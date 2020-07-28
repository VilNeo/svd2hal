use super::super::input;
use super::templates;

use serde_derive::Serialize;
use std::fs;
use std::fs::File;
use std::io::Write;

pub struct Output {
    pub svd: patch_svd::Svd,
}

impl Output {
    pub fn from(input: &mut input::Input) -> Output {
        Output {
            svd: input.svd.clone(),
        }
    }

    pub fn write(&self, output_path: String) {
        let device_name = self.svd.device.name.clone();
        // create project directory
        let project_name = "rawhal-".to_string() + device_name.to_ascii_lowercase().as_str();
        let project_dir_path = output_path + project_name.as_str();
        if fs::create_dir(project_dir_path.clone()).is_err() {
            //panic!("Could not create project directory {}. Please check permissions, path and make sure that the project dir does not exist already.", project_dir_path);
        }
        //      create Cargo.toml
        {
            #[derive(Serialize)]
            struct Content {
                project_name: String,
            }
            let content = Content { project_name };
            templates::render_template_into_path(
                templates::CARGO_TOML_TEMPLATE,
                &content,
                &(project_dir_path.clone() + "/Cargo.toml"),
            );
        }
        //      peripheral.x
        templates::render_template_into_path(
            templates::LINKER_TEMPLATE,
            &self.svd.device,
            &(project_dir_path.clone() + "/peripheral.x"),
        );
        //      create src directory
        let src_dir_path = project_dir_path.clone() + "/src";
        if src_dir_path.ends_with("rawhal-stm32l4x2/src") {
            //fs::remove_dir(src_dir_path.clone())
            //.expect("Could not remove previously created hal project src directory.");
        }
        if fs::create_dir(&src_dir_path).is_err() {
            //panic!("Could not create project directory {}. Please check permissions, path and make sure that the project dir does not exist already.", project_dir_path);
        }
        //              Macros.rs
        let macro_file_content = include_bytes!("macros.rs");
        let mut macro_file =
            File::create(src_dir_path.clone() + &"/macros.rs".to_string()).unwrap();
        macro_file.write(macro_file_content).unwrap();

        //              Peripheral files
        templates::render_template_into_path(
            templates::PERIPHERALS_TEMPLATE,
            &self.svd.device,
            &(src_dir_path.clone() + "/peripherals.rs"),
        );
    }
}
