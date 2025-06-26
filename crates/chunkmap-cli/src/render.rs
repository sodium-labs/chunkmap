use anvilregion::{
    dimensions::Dimension,
    regions::{parse_region_bytes, Region},
};
use chunkmap::images::{create_region_images, ImageRenderType};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::{
    collections::VecDeque,
    fs::{create_dir_all, read_dir, File},
    io::{self, Read},
    path::Path,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

fn parse_region_file(path: &Path) -> io::Result<Region> {
    let mut file = File::open(path)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;
    parse_region_bytes(&data)
}

pub fn render_regions(
    input_path: &str,
    output_path: &str,
    render_type: ImageRenderType,
    dimension: Dimension,
) {
    let region_path = Path::new(input_path);

    let mca_files: Vec<_> = read_dir(region_path)
        .expect("Failed to read region folder")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().map(|ext| ext == "mca").unwrap_or(false)
                && path.metadata().map(|meta| meta.len() > 0).unwrap_or(false)
        })
        .collect();
    let file_count = mca_files.len();

    let file_queue = Arc::new(Mutex::new(VecDeque::from(mca_files)));
    let mut handles = Vec::new();

    create_dir_all(output_path).unwrap();

    let num_threads = thread::available_parallelism().unwrap().get();

    // START: Progress bar

    let m = MultiProgress::new();

    let status_bar = m.add(ProgressBar::new_spinner());
    status_bar.set_style(
        ProgressStyle::with_template("[Thread {prefix}]: {spinner:.green} Rendering {msg}")
            .unwrap(),
    );
    status_bar.enable_steady_tick(Duration::from_millis(100));

    let main_bar = m.add(ProgressBar::new(file_count as u64));
    main_bar.set_style(ProgressStyle::with_template("[{bar:40.cyan/blue}] {pos}/{len}").unwrap());

    let main_bar = Arc::new(main_bar);
    let status_bar = Arc::new(status_bar);

    // END: Progress bar

    for thread_idx in 0..num_threads {
        let main_bar = Arc::clone(&main_bar);
        let status_bar = Arc::clone(&status_bar);
        let file_queue = Arc::clone(&file_queue);

        let output_path = output_path.to_string().clone();
        let dimension = dimension.clone();
        let render_type = render_type.clone();

        let handle = thread::spawn(move || loop {
            let path_opt = {
                let mut queue = file_queue.lock().unwrap();
                queue.pop_front()
            };

            let path = match path_opt {
                Some(p) => p,
                None => break,
            };

            status_bar.set_prefix(format!("{}/{}", thread_idx, num_threads));
            status_bar.set_message(format!("{:?}", path.file_name().unwrap()));

            match parse_region_file(&path) {
                Ok(region) => {
                    match create_region_images(&region.chunks, &dimension, &render_type) {
                        Ok(imgs) => {
                            for (rx, rz, img) in imgs {
                                let filename =
                                    format!("{}/r.{}.{}.png", output_path.clone(), rx, rz);
                                img.save(&filename).unwrap();
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to create region image: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to parse region: {}", e);
                }
            }

            main_bar.inc(1);
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    main_bar.finish_with_message("Rendering finished");
    status_bar.finish_and_clear();
}
