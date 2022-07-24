use std::io::{Error, ErrorKind, stderr, Write};
use std::path::{Path, PathBuf};
use std::process::exit;
use std::sync::{Arc, Mutex, MutexGuard};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

use console::Color::Color256;
use console::{style, Style};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::{ThreadPool, ThreadPoolBuilder};

use directories as dirs;

use crate::commands::transcode::packets::album::AlbumWorkPacket;
use crate::commands::transcode::packets::file::FileWorkPacket;
use crate::commands::transcode::packets::library::LibraryWorkPacket;
use crate::configuration::Config;
use crate::console as c;

mod meta;
mod directories;
mod packets;

const DEFAULT_PROGRESS_BAR_TICK_INTERVAL: Duration = Duration::from_millis(100);

/// Builds a ProgressStyle that contains the requested header at the beginning of the line.
fn build_progress_bar_style_with_header<S: AsRef<str>>(header_str: S) -> ProgressStyle {
    ProgressStyle::with_template(
        &format!(
            "{}{}",
            header_str.as_ref(),
            "{msg:^42!} [{elapsed_precise} | {pos:>3}/{len:3}] [{bar:45.cyan/blue}]"
        ),
    )
        .unwrap()
        .progress_chars("#>-")
}

/// A HOF (Higher-order-function) that takes a ProgressBar reference and a text Style and
/// *returns* a function that will then always take a single parameter: the text to set on the progress bar.
fn build_styled_progress_bar_message_fn(
    progress_bar: &ProgressBar,
    text_style: Style,
) -> impl Fn(&str) + Send + Clone {
    let progress_bar = progress_bar.clone();

    move |text: &str| {
        progress_bar.set_message(
            format!(
                "{}",
                text_style.apply_to(text),
            ),
        );
    }
}

/// This is a higher-order-function. It is similar to `build_styled_progress_bar_message_fn`,
/// but instead builds and return a function that will take two parameters:
/// the text to set, and the progress bar to set it to.
/// Importantly, the second parameter should be behind a MutexGuard reference
/// (meaning that we have it locked at call time).
fn build_styled_progress_bar_message_fn_dynamic_locked_bar(
    text_style: Style,
) -> impl Fn(&str, &MutexGuard<ProgressBar>) + Send + Clone {
    move |text: &str, progress_bar: &MutexGuard<ProgressBar>| {
        progress_bar.set_message(
            format!(
                "{}",
                text_style.apply_to(text),
            ),
        );
    }
}

/// Builds a ThreadPool using the `transcode_threads` configuration value.
fn build_transcode_thread_pool(config: &Config) -> ThreadPool {
    ThreadPoolBuilder::new()
        .num_threads(config.aggregated_library.transcode_threads as usize)
        .build()
        .unwrap()
}

/// Processes all given `FileWorkPacket`s in parallel as allowed by the given ThreadPool.
/// Updates the progress bar after each successful step.
fn process_file_packets_in_threadpool<F: Fn(&str, &MutexGuard<ProgressBar>) + Send + Clone>(
    config: &Config,
    thread_pool: &ThreadPool,
    file_packets: Vec<FileWorkPacket>,
    file_progress_bar_arc: &Arc<Mutex<ProgressBar>>,
    file_progress_bar_set_fn: F,
) -> Result<(), Vec<Error>> {
    if file_packets.len() == 0 {
        return Ok(());
    }

    let fpb_threadpool_clone = file_progress_bar_arc.clone();
    let (tx, rx): (Sender<Error>, Receiver<Error>) = channel();

    let progress_bar_callback = file_progress_bar_set_fn.clone();

    thread_pool.scope(move |s| {
        for file_packet in file_packets {
            let thread_tx = tx.clone();
            let thread_progress_bar = fpb_threadpool_clone.clone();
            let thread_pbc = progress_bar_callback.clone();

            let file_name = match file_packet.get_file_name() {
                Ok(name) => name,
                Err(error) => {
                    thread_tx.send(error)
                        .expect("Work thread could not send file name error to main thread.");
                    return;
                }
            };

            s.spawn(move |_| {
                let work_result = file_packet.process(config);

                let thread_progress_bar_lock = thread_progress_bar.lock().unwrap();
                thread_progress_bar_lock.inc(1);
                thread_pbc(
                    &file_name,
                    &thread_progress_bar_lock,
                );

                if work_result.is_err() {
                    thread_tx.send(work_result.unwrap_err())
                        .expect("Work thread could not send error to main thread!");
                }
            });
        }
    });

    let collected_thread_errors: Vec<Error> = Vec::from_iter(rx.try_iter());
    return if collected_thread_errors.len() == 0 {
        Ok(())
    } else {
        Err(collected_thread_errors)
    }
}

/// Just a handy shortcut for printing a Vec of Errors when one or more worker threads fail.
/// Always returns Err(Error).
fn print_error_vector_and_return_err(errors: Vec<Error>) -> Result<(), Error> {
    eprintln!(
        "{}",
        style("Something went wrong with one or more worker threads:")
            .red(),
    );
    for err in errors {
        eprintln!("  {}", err);
    }
    stderr().flush().expect("Could not flush stderr.");

    return Err(
        Error::new(
            ErrorKind::Other,
            "One or more transcoding threads errored.",
        ),
    );
}


/// This function lists all the albums in all of the libraries that need to be transcoded
/// and performs the transcode using ffmpeg (for audio files) and simple file copy (for data files).
pub fn cmd_transcode_all(config: &Config) -> Result<(), Error> {
    c::horizontal_line_with_text(
        format!(
            "{}",
            style("transcoding (all libraries)")
                .cyan()
                .bold()
        ),
        None, None,
    );
    c::new_line();

    let processing_begin_time = Instant::now();

    println!(
        "{}",
        style("Scanning libraries for changes...")
            .yellow()
            .bright(),
    );

    // List all libraries.
    let mut library_packets: Vec<LibraryWorkPacket> = Vec::new();
    for (library_name, library) in &config.libraries {
        library_packets.push(
            LibraryWorkPacket::from_library_path(
                library_name,
                Path::new(&library.path),
                config,
            )?,
        );
    }

    let total_libraries = library_packets.len();

    // Filter libraries to ones that need at least one album processed.
    let mut filtered_library_packets: Vec<(LibraryWorkPacket, Vec<AlbumWorkPacket>)> = Vec::new();
    for mut library_packet in library_packets {
        // Not all albums need to be processed each time, this function returns only the list
        // of albums that need are either mising or have changed according to the .librarymeta file.
        let mut albums_in_need_of_processing = library_packet.get_albums_in_need_of_processing(config)?;
        albums_in_need_of_processing.sort_unstable_by(
            |first, second| {
                first.album_info.album_title.cmp(&second.album_info.album_title)
            }
        );

        if albums_in_need_of_processing.len() > 0 {
            filtered_library_packets.push((library_packet, albums_in_need_of_processing));
        }
    }

    filtered_library_packets.sort_unstable_by(
        |(first, _), (second, _)| {
            first.name.cmp(&second.name)
        }
    );

    let total_filtered_libraries = filtered_library_packets.len();
    if total_filtered_libraries == 0 {
        println!(
            "{}",
            style("All transcodes are already up to date.")
                .green()
                .bright()
                .bold(),
        );
        return Ok(());
    } else {
        println!(
            "{}/{} libraries need transcoding:",
            style(total_filtered_libraries)
                .bold()
                .italic(),
            style(total_libraries)
                .bold(),
        );
        for (library, albums) in &filtered_library_packets {
            println!(
                "  {:20} {} new or changed albums.",
                style(format!("{}:", library.name))
                    .yellow()
                    .italic(),
                style(albums.len())
                    .bold()
            );
        }
        c::new_line();
    }

    // Set up progress bars (three bars, one for current file, another for albums, the third for libraries).
    let multi_pbr = MultiProgress::new();

    let files_progress_bar = multi_pbr.add(ProgressBar::new(0));
    files_progress_bar.set_style(
        build_progress_bar_style_with_header(format!("{:9}", "(file)")),
    );
    files_progress_bar.enable_steady_tick(DEFAULT_PROGRESS_BAR_TICK_INTERVAL);

    let files_progress_bar_ref = Arc::new(Mutex::new(files_progress_bar));

    let albums_progress_bar = multi_pbr.add(ProgressBar::new(0));
    albums_progress_bar.set_style(
        build_progress_bar_style_with_header(format!("{:9}", "(album)")),
    );
    albums_progress_bar.enable_steady_tick(DEFAULT_PROGRESS_BAR_TICK_INTERVAL);

    let library_progress_bar = multi_pbr.add(ProgressBar::new(filtered_library_packets.len() as u64));
    library_progress_bar.set_style(
        build_progress_bar_style_with_header(format!("{:9}", "(library)")),
    );
    library_progress_bar.enable_steady_tick(DEFAULT_PROGRESS_BAR_TICK_INTERVAL);

    // TODO Manually truncate names that are too long (42), automatic truncation trims only the colours.
    // TODO If the user interrupts a transcode, ask if they want to delete the currently half-transcoded album.

    let set_current_file = build_styled_progress_bar_message_fn_dynamic_locked_bar(
        Style::new().fg(Color256(131)).underlined(),
    );

    let set_current_album = build_styled_progress_bar_message_fn(
        &albums_progress_bar,
        Style::new().fg(Color256(131)).underlined(),
    );

    let set_current_library = build_styled_progress_bar_message_fn(
        &library_progress_bar,
        Style::new().white().underlined(),
    );

    set_current_file("/", &files_progress_bar_ref.lock().unwrap());
    set_current_album("/");
    set_current_library("/");

    let thread_pool = build_transcode_thread_pool(config);

    // Iterate over libraries and process each album.
    for (library, album_packets) in filtered_library_packets {
        set_current_library(&library.name);

        albums_progress_bar.reset();
        albums_progress_bar.set_length(album_packets.len() as u64);
        albums_progress_bar.set_position(0);

        for mut album_packet in album_packets {
            set_current_album(&album_packet.album_info.album_title);

            let file_packets = album_packet.get_work_packets(config)?;

            {
                let fpb_locked = files_progress_bar_ref.lock().unwrap();
                fpb_locked.reset();
                fpb_locked.set_length(file_packets.len() as u64);
                fpb_locked.set_position(0);
            }

            match process_file_packets_in_threadpool(
                config,
                &thread_pool,
                file_packets,
                &files_progress_bar_ref,
                set_current_file.clone(),
            ) {
                Ok(()) => (),
                Err(errors) => {
                    return print_error_vector_and_return_err(errors);
                }
            };

            album_packet.save_fresh_meta(config, true)?;
            albums_progress_bar.inc(1);
        }

        library_progress_bar.inc(1);
    }

    files_progress_bar_ref.lock().unwrap().finish();
    albums_progress_bar.finish();
    library_progress_bar.finish();

    let processing_time_delta = processing_begin_time.elapsed();
    println!(
        "Transcoding completed in {:.1?}.",
        processing_time_delta,
    );

    // TODO Check why sometimes the process fails with "The system cannot find the path specified. (os error 3)"
    Ok(())
}

/// This function lists all the allbums in the selected library that need to be transcoded
/// and performs the actual transcode using ffmpeg (for audio files) and simple file copy (for data files).
pub fn cmd_transcode_library(library_directory: &PathBuf, config: &Config) -> Result<(), Error> {
    if !library_directory.is_dir() {
        println!("Directory is invalid.");
        exit(1);
    }

    c::horizontal_line_with_text(
        format!(
            "{}",
            style("transcoding (single library)")
                .cyan()
                .bold(),
        ),
        None, None,
    );
    c::new_line();

    let processing_begin_time = Instant::now();

    let library_directory_string = library_directory.to_string_lossy().to_string();
    println!(
        "{} {}",
        style("Library directory: ")
            .italic(),
        library_directory_string,
    );
    c::new_line();

    if !config.is_library(library_directory) {
        println!(
            "{}",
            style("Selected directory is not a registered library, exiting.")
                .red(),
        );

        exit(1);
    }

    println!(
        "{}",
        style("Scanning library for changes...")
            .yellow()
            .bright(),
    );

    let library_name = config.get_library_name_from_path(library_directory)
        .ok_or(Error::new(ErrorKind::Other, "No registered library."))?;

    let mut library_packet = LibraryWorkPacket::from_library_path(
        &library_name,
        library_directory,
        config,
    )?;

    // Filter to just the albums that need to be processed.
    let mut filtered_album_packets = library_packet.get_albums_in_need_of_processing(config)?;
    let total_filtered_albums = filtered_album_packets.len();

    if total_filtered_albums == 0 {
        println!(
            "{}",
            style("Transcodes of this library are already up to date.")
                .green()
                .bright()
                .bold(),
        );
        return Ok(());
    } else {
        println!(
            "{}/{} albums in this library are new or have changed.",
            style(total_filtered_albums)
                .bold()
                .underlined(),
            style(library_packet.album_packets.len())
                .bold(),
        );
        c::new_line();
    }

    // Set up two progress bars, one for the current file, another for the current album.
    let multi_pbr = MultiProgress::new();

    let file_progress_bar = multi_pbr.add(ProgressBar::new(0));
    file_progress_bar.set_style(
        build_progress_bar_style_with_header(format!("{:9}", "(file)")),
    );
    file_progress_bar.enable_steady_tick(DEFAULT_PROGRESS_BAR_TICK_INTERVAL);

    let album_progress_bar = multi_pbr.add(ProgressBar::new(filtered_album_packets.len() as u64));
    album_progress_bar.set_style(
        build_progress_bar_style_with_header(format!("{:9}", "(album)")),
    );
    album_progress_bar.enable_steady_tick(DEFAULT_PROGRESS_BAR_TICK_INTERVAL);

    let file_progress_bar_ref = Arc::new(Mutex::new(file_progress_bar));


    let set_current_file = build_styled_progress_bar_message_fn_dynamic_locked_bar(
        Style::new().fg(Color256(131)).underlined(),
    );

    let set_current_album = build_styled_progress_bar_message_fn(
        &album_progress_bar,
        Style::new().fg(Color256(103)).underlined(),
    );


    set_current_file("/", &file_progress_bar_ref.lock().unwrap());
    set_current_album("/");

    let thread_pool = build_transcode_thread_pool(config);

    // Transcode all albums that are new or have changed.
    for album_packet in &mut filtered_album_packets {
        set_current_album(&album_packet.album_info.album_title);

        let file_work_packets = album_packet.get_work_packets(config)?;

        {
            let fpb_lock = file_progress_bar_ref.lock().unwrap();
            fpb_lock.reset();
            fpb_lock.set_length(file_work_packets.len() as u64);
            fpb_lock.set_position(0);
        }

        match process_file_packets_in_threadpool(
            config,
            &thread_pool,
            file_work_packets,
            &file_progress_bar_ref,
            set_current_file.clone(),
        ) {
            Ok(()) => (),
            Err(errors) => {
                return print_error_vector_and_return_err(errors);
            }
        }

        album_packet.save_fresh_meta(config, true)?;
        album_progress_bar.inc(1);
    }

    file_progress_bar_ref.lock().unwrap().finish();
    album_progress_bar.finish();

    let processing_time_delta = processing_begin_time.elapsed();
    println!(
        "Library transcoded in {:.1?}.",
        processing_time_delta,
    );

    Ok(())
}

/// This function transcodes a single album using ffmpeg (for audio files) and simple file copy (for data files).
pub fn cmd_transcode_album(album_directory: &Path, config: &Config) -> Result<(), Error> {
    if !album_directory.is_dir() {
        println!("Directory is invalid.");
        exit(1);
    }

    c::horizontal_line_with_text(
        format!(
            "{}",
            style("transcoding (single album)")
                .cyan()
                .bold(),
        ),
        None, None,
    );
    c::new_line();

    let processing_begin_time = Instant::now();

    let album_directory_string = album_directory.to_string_lossy().to_string();
    println!(
        "{} {}",
        style("Album directory: ")
            .italic(),
        album_directory_string,
    );
    c::new_line();

    // Verify the directory is an album.
    if !dirs::directory_is_album(config, album_directory) {
        eprintln!(
            "{}",
            style("Not an album directory, exiting.")
                .red()
        );

        exit(1);
    }

    println!(
        "{}",
        style("Scanning album...")
            .yellow()
            .bright(),
    );

    let mut album_packet = AlbumWorkPacket::from_album_path(album_directory, config)?;
    let total_track_count = album_packet.get_total_track_count(config)?;
    let file_packets = album_packet.get_work_packets(config)?;

    println!(
        "{}/{} files in this album are new or have changed.",
        style(file_packets.len())
            .bold()
            .underlined(),
        style(total_track_count)
            .bold(),
    );
    c::new_line();

    if file_packets.len() == 0 {
        println!(
            "{}",
            style("Transcoded album is already up to date.")
                .green()
                .bright()
                .bold(),
        );

        return Ok(());
    }

    // Set up a progress bar for the current file.
    let file_progress_bar = ProgressBar::new(file_packets.len() as u64);
    file_progress_bar.set_style(
        build_progress_bar_style_with_header(format!("{:9}", "(file)")),
    );
    file_progress_bar.enable_steady_tick(DEFAULT_PROGRESS_BAR_TICK_INTERVAL);

    let file_progress_bar_arc = Arc::new(Mutex::new(file_progress_bar));

    let set_current_file = build_styled_progress_bar_message_fn_dynamic_locked_bar(
        Style::new().fg(Color256(131)).underlined(),
    );


    let thread_pool = build_transcode_thread_pool(config);

    match process_file_packets_in_threadpool(
        config,
        &thread_pool,
        file_packets,
        &file_progress_bar_arc,
        set_current_file.clone(),
    ) {
        Ok(()) => (),
        Err(errors) => {
            return print_error_vector_and_return_err(errors);
        }
    };

    album_packet.save_fresh_meta(config, true)?;

    file_progress_bar_arc.lock().unwrap().finish();

    let processing_time_delta = processing_begin_time.elapsed();
    println!(
        "Album transcoded in {:.1?}.",
        processing_time_delta,
    );

    Ok(())
}
