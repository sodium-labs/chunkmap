use anvilregion::{dimensions::Dimension, regions::parse_region_bytes};
use chunkmap::images::{create_region_images, ImageRenderType};
use console_error_panic_hook;
use image::ImageFormat;
use serde::Serialize;
use std::io::Cursor;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
pub fn initialize() {
    console_error_panic_hook::set_once();
    log("[WASM] Loaded");
}

#[derive(Serialize)]
struct JSRegion {
    x: i32,
    z: i32,
    buffer: Vec<u8>,
}

#[wasm_bindgen]
pub fn parse_region_file(bytes: &[u8]) -> Result<Vec<JsValue>, JsError> {
    match parse_region_bytes(bytes) {
        Ok(region) => match create_region_images(
            &region.chunks,
            &Dimension::Overworld,
            &ImageRenderType::Textures,
        ) {
            Ok(imgs) => {
                let mut values = Vec::new();

                for (x, z, img) in imgs {
                    let mut buffer = Vec::new();

                    log(&format!("Creating image {x}.{z}"));

                    img.write_to(&mut Cursor::new(&mut buffer), ImageFormat::Png)
                        .unwrap();

                    let region = JSRegion { x, z, buffer };
                    values.push(serde_wasm_bindgen::to_value(&region)?);
                }

                Ok(values)
            }
            Err(e) => {
                let error_str = &format!("Failed to create region images: {e:#?}");
                log(&error_str);
                Err(JsError::new(&error_str))
            }
        },
        Err(e) => Err(e.into()),
    }
}
