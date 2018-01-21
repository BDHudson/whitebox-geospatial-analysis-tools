/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 2, 2017
Last Modified: January 21, 2018
License: MIT
*/
extern crate time;
extern crate num_cpus;

use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::path;
use lidar::*;
use raster::*;
use tools::*;

pub struct BlockMaximum {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl BlockMaximum {
    pub fn new() -> BlockMaximum {
        // public constructor
        let name = "BlockMaximum".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description = "Creates a block-maximum raster from an input LAS file.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter{
            name: "Input LiDAR File".to_owned(), 
            flags: vec!["-i".to_owned(), "--input".to_owned()], 
            description: "Input LiDAR file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Lidar),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Output File".to_owned(), 
            flags: vec!["-o".to_owned(), "--output".to_owned()], 
            description: "Output file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Grid Resolution".to_owned(), 
            flags: vec!["--resolution".to_owned()], 
            description: "Output raster's grid resolution.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("1.0".to_owned()),
            optional: true
        });

        parameters.push(ToolParameter{
            name: "Palette Name (Whitebox raster outputs only)".to_owned(), 
            flags: vec!["--palette".to_owned()], 
            description: "Optional palette name (for use with Whitebox raster files).".to_owned(),
            parameter_type: ParameterType::String,
            default_value: None,
            optional: true
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "")
            .replace(".exe", "")
            .replace(".", "")
            .replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=file.las -o=outfile.dep --resolution=2.0\"
.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=file.las -o=outfile.dep --resolution=5.0 --palette=light_quant.plt", short_exe, name).replace("*", &sep);

        BlockMaximum {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for BlockMaximum {
    fn get_source_file(&self) -> String {
        String::from(file!())
    }
    
    fn get_tool_name(&self) -> String {
        self.name.clone()
    }

    fn get_tool_description(&self) -> String {
        self.description.clone()
    }

    fn get_tool_parameters(&self) -> String {
        let mut s = String::from("{\"parameters\": [");
        for i in 0..self.parameters.len() {
            if i < self.parameters.len() - 1 {
                s.push_str(&(self.parameters[i].to_string()));
                s.push_str(",");
            } else {
                s.push_str(&(self.parameters[i].to_string()));
            }
        }
        s.push_str("]}");
        s
    }

    fn get_example_usage(&self) -> String {
        self.example_usage.clone()
    }

    fn get_toolbox(&self) -> String {
        self.toolbox.clone()
    }

    fn run<'a>(&self,
               args: Vec<String>,
               working_directory: &'a str,
               verbose: bool)
               -> Result<(), Error> {
        let mut input_file: String = "".to_string();
        let mut output_file: String = "".to_string();
        let mut grid_res: f64 = 1.0;
        let mut palette = "default".to_string();

        // read the arguments
        if args.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidInput,
                                  "Tool run with no paramters."));
        }
        for i in 0..args.len() {
            let mut arg = args[i].replace("\"", "");
            arg = arg.replace("\'", "");
            let cmd = arg.split("="); // in case an equals sign was used
            let vec = cmd.collect::<Vec<&str>>();
            let mut keyval = false;
            if vec.len() > 1 {
                keyval = true;
            }
            let flag_val = vec[0].to_lowercase().replace("--", "-");
            if flag_val == "-i" || flag_val == "-input" {
                input_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-resolution" {
                grid_res = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
            } else if flag_val == "-palette" {
                palette = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            }
        }

        if !input_file.contains(path::MAIN_SEPARATOR) {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(path::MAIN_SEPARATOR) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let start = time::now();

        if verbose {
            println!("Reading input LAS file...");
        }
        let input = match LasFile::new(&input_file, "r") {
            Ok(lf) => lf,
            Err(_) => {
                return Err(Error::new(ErrorKind::NotFound,
                                      format!("No such file or directory ({})", input_file)))
            }
        };

        let n_points = input.header.number_of_points as usize;
        let num_points: f64 = (input.header.number_of_points - 1) as f64; // used for progress calculation only

        if verbose {
            println!("Performing analysis...");
        }
        let west: f64 = input.header.min_x; // - 0.5 * grid_res;
        let north: f64 = input.header.max_y; // + 0.5 * grid_res;
        let rows: usize = (((north - input.header.min_y) / grid_res).ceil()) as usize;
        let columns: usize = (((input.header.max_x - west) / grid_res).ceil()) as usize;
        let south: f64 = north - rows as f64 * grid_res;
        let east = west + columns as f64 * grid_res;
        let nodata = -32768.0f64;
        let half_grid_res = grid_res / 2.0;
        let ns_range = north - south;
        let ew_range = east - west;

        let mut configs = RasterConfigs { ..Default::default() };
        configs.rows = rows;
        configs.columns = columns;
        configs.north = north;
        configs.south = south;
        configs.east = east;
        configs.west = west;
        configs.resolution_x = grid_res;
        configs.resolution_y = grid_res;
        configs.nodata = nodata;
        configs.data_type = DataType::F64;
        configs.photometric_interp = PhotometricInterpretation::Continuous;
        configs.palette = palette;

        let mut output = Raster::initialize_using_config(&output_file, &configs);

        let input = Arc::new(input); // wrap input in an Arc
        let num_procs = num_cpus::get();
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut col: isize;
                let mut row: isize;
                for i in (0..n_points).filter(|point_num| point_num % num_procs == tid) {
                    let p: PointData = input.get_point_info(i);
                    col = (((columns - 1) as f64 * (p.x - west - half_grid_res) / ew_range)
                               .round()) as isize;
                    row = (((rows - 1) as f64 * (north - half_grid_res - p.y) / ns_range)
                               .round()) as isize;
                    tx.send((row, col, p.z)).unwrap();
                }
            });
        }

        let mut col: isize;
        let mut row: isize;
        let mut z: f64;
        let mut progress: i32;
        let mut old_progress: i32 = 1;
        for i in 0..n_points {
            let data = rx.recv().unwrap();
            row = data.0;
            col = data.1;
            z = data.2;
            if output[(row, col)] == nodata || z > output[(row, col)] {
                output.set_value(row, col, z);
            }
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let end = time::now();
        let elapsed_time = end - start;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool",
                                          self.get_tool_name()));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time)
                                      .replace("PT", ""));

        if verbose {
            println!("Saving data...")
        };
        let _ = match output.write() {
            Ok(_) => {
                if verbose {
                    println!("Output file written")
                }
            }
            Err(e) => return Err(e),
        };

        println!("{}",
                 &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        Ok(())
    }
}
