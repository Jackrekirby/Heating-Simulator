use std::{env};
use std::fs::File;
use std::io::{prelude::*, BufReader, LineWriter};
use std::ops::Range;
use std::time::Instant;

mod heat_ninja;

struct Inputs {
    thermostat_temperature: f32,
    latitude: f32,
    longitude: f32,
    num_occupants: u8,
    house_size: f32,
    postcode: String,
    epc_space_heating: f32,
    tes_volume_max: f32,
}

fn import_file_data(
    assets_dir: String,
    latitude: f32,
    longitude: f32,
) -> ([f32; 8760], [f32; 8760], [f32; 8760]) {
    // wasm does not support Rust file io,
    // -> file data must be gathered beforehand then passed into wasm functions

    let import_weather_data = |filepath: String| -> [f32; 8760] {
        //println!("filepath: {}", filepath);
        let file = File::open(&filepath).expect(&format!("cannot read file: {}", &filepath));
        let reader = BufReader::new(file);
        let mut data: [f32; 8760] = [0.0; 8760];
        for (i, line) in reader.lines().enumerate() {
            data[i] = line.unwrap().parse::<f32>().expect("string invalid float.");
        }
        data
    };

    let format_coordinate = |coordinate: f32| -> f32 {
        let rounded: i16 = (coordinate * 2.0).round() as i16;
        if rounded == -0 {
            0.0
        } else {
            (rounded as f32) / 2.0
        }
    };

    let build_weather_file_path = |datatype: &str| -> String {
        format!(
            "{}{}/lat_{:.1}_lon_{:.1}.csv",
            assets_dir,
            datatype,
            format_coordinate(latitude),
            format_coordinate(longitude)
        )
    };

    let agile_tariff_per_hour_over_year: [f32; 8760] =
        import_weather_data(format!("{}agile_tariff.csv", assets_dir));

    let hourly_outside_temperatures_over_year: [f32; 8760] =
        import_weather_data(build_weather_file_path("outside_temps"));

    let hourly_solar_irradiances_over_year: [f32; 8760] =
        import_weather_data(build_weather_file_path("solar_irradiances"));

    (
        agile_tariff_per_hour_over_year,
        hourly_outside_temperatures_over_year,
        hourly_solar_irradiances_over_year,
    )
}

#[allow(dead_code)]
fn run_simulation(inputs: &Inputs, config: &heat_ninja::Config) {
    let (
        agile_tariff_per_hour_over_year,
        hourly_outside_temperatures_over_year,
        hourly_solar_irradiances_over_year,
    ) = import_file_data(String::from("assets/"), inputs.latitude, inputs.longitude);

    let json_results = heat_ninja::run_simulation(
        inputs.thermostat_temperature,
        inputs.latitude,
        inputs.longitude,
        inputs.num_occupants,
        inputs.house_size,
        &inputs.postcode,
        inputs.epc_space_heating,
        inputs.tes_volume_max,
        &agile_tariff_per_hour_over_year,
        &hourly_outside_temperatures_over_year,
        &hourly_solar_irradiances_over_year,
        &config,
    );

    if config.return_format != heat_ninja::ReturnFormat::NONE {
        println!("{}", json_results);
    }
}

#[allow(dead_code)]
fn run_simulations_using_input_file() {
    // import input list file
    // postcode, latitude, longitude, num_occupants, house_size, temp, epc_space_heating, tes_max
    let filepath = "tests/input_list.csv";
    println!("filepath: {}", filepath);
    let file = File::open(filepath).expect("cannot read file.");
    let reader = BufReader::new(file);
    let all_now = Instant::now();

    let file = File::create("tests/performance.csv").expect("could not open tests/performance.csv");
    let mut file: LineWriter<File> = LineWriter::new(file);

    file.write_fmt(format_args!("Index,Nodes,Gain,Opt-Elapsed,No-Opt-Elapsed,No-Opt-Elapsed/Node\n")).expect("could not write to tests/performance.csv");

    for (i, line) in reader.lines().enumerate() {
        if i < 28 {continue;}
        let just_now = Instant::now();
        let unwrapped_line = line.unwrap();
        let parts: Vec<&str> = unwrapped_line.split(',').collect();

        let postcode: String = String::from(parts[0]);
        let latitude: f32 = parts[1].parse().unwrap();
        let longitude: f32 = parts[2].parse().unwrap();
        let num_occupants: u8 = parts[3].parse().unwrap();
        let house_size: f32 = parts[4].parse().unwrap();
        let tes_volume_max: f32 = parts[7].parse().unwrap();
        let thermostat_temperature: f32 = parts[5].parse().unwrap();
        let epc_space_heating: f32 = parts[6].parse().unwrap();

        println!("Index: {}, Inputs: {:?}", i, parts);

        let inputs = Inputs {
            postcode,
            latitude,
            longitude,
            num_occupants,
            house_size,
            tes_volume_max,
            thermostat_temperature,
            epc_space_heating,
        };

        run_and_compare(inputs, i, &mut file);

        println!(
            "Elapsed: {} ms, Total: {} s",
            just_now.elapsed().as_millis(),
            all_now.elapsed().as_secs()
        );
    }
}

#[allow(dead_code)]
fn compare_result_folders(file_name_fmt_a: fn(usize) -> String, file_name_fmt_b: fn(usize) -> String, index_range: Range<usize>) {
    for file_index in index_range {
        println!("File Index: {}", file_index);
        let file_name_a: String = file_name_fmt_a(file_index);
        let file_name_b: String = file_name_fmt_b(file_index);

        compare_result_files(file_name_a, file_name_b, file_index);
    }
}

#[allow(dead_code)]
fn compare_result_files(file_name_a: String, file_name_b: String, file_index: usize) {
    let file_a = File::open(&file_name_a).expect("cannot read file.");
    let reader_a = BufReader::new(&file_a);

    let file_b = File::open(&file_name_b).expect("cannot read file.");
    let reader_b = BufReader::new(&file_b);

    for ((line_index, result_a), result_b) in reader_a.lines().enumerate().zip(reader_b.lines()) {
        //println!("{}, {}, {}", line_index, line_a.unwrap(), line_b.unwrap());
        let line_a = result_a.expect(&format!("could not read line: {} of {}", line_index, &file_name_a));
        let line_b = result_b.expect(&format!("could not read line: {} of {}", line_index, &file_name_b));
        if line_a != line_b {
            panic!("line {} of files: {}, {}, do not match: \n{}\n{}\n. Surface: {}",
                line_index, file_name_a, file_name_b, line_a, line_b, file_index * 21 + line_index - 3,
            );
        }
    }
}

#[allow(dead_code)]
fn run_simulation_with_default_parameters() {
    let config: heat_ninja::Config = heat_ninja::Config {
        print_intermediates: false,
        print_results_as_csv: false,
        use_surface_optimisation: true,
        use_multithreading: true,
        file_index: 0,
        save_results_as_csv: true,
        save_results_as_json: false,
        print_results_as_json: false,
        return_format: heat_ninja::ReturnFormat::NONE,
        save_surfaces: true
    };

    let inputs = Inputs {
        thermostat_temperature: 20.0,
        latitude: 52.3833, longitude: -1.5833,
        num_occupants: 2, house_size: 360.0,
        postcode: String::from("CV4 7AL"),
        epc_space_heating: 3000.0, tes_volume_max: 3.0
    };

    run_simulation(&inputs, &config);
}

#[allow(dead_code)]
fn run_and_compare(inputs: Inputs, file_index: usize, file: &mut LineWriter<File>) {
    let mut config: heat_ninja::Config = heat_ninja::Config {
        print_intermediates: false,
        print_results_as_csv: false,
        use_surface_optimisation: true,
        use_multithreading: true,
        file_index: file_index,
        save_results_as_csv: true,
        save_results_as_json: false,
        print_results_as_json: false,
        return_format: heat_ninja::ReturnFormat::NONE,
        save_surfaces: true
    };

    let total_nodes: u16 = ((180 * ((inputs.house_size / 8.0) as u16) - 30) as f32 * inputs.tes_volume_max) as u16;

    let now = Instant::now();
    run_simulation(&inputs, &config);
    let opt_elapsed: u128 = now.elapsed().as_millis();

    let now = Instant::now();
    config.use_surface_optimisation = false;
    run_simulation(&inputs, &config);
    let no_opt_elapsed: u128 = now.elapsed().as_millis();

    println!("Nodes: {:.0}, % Gain: {:.2}, Elapsed: {} vs {} ms, Elapsed/Node: {} ms", total_nodes,
             no_opt_elapsed as f32 / opt_elapsed as f32, opt_elapsed, no_opt_elapsed, no_opt_elapsed as f32 / total_nodes as f32);

    file.write_fmt(format_args!(
        "{},{:.0},{:.2},{},{},{:.4}\n", file_index, total_nodes,
        no_opt_elapsed as f32 / opt_elapsed as f32,
        opt_elapsed, no_opt_elapsed,
        no_opt_elapsed as f32 / total_nodes as f32))
       .expect("could not write to tests/performance.csv");

    compare_result_files(
        String::from(format!("tests/results/o{}.csv", file_index)),
        String::from(format!("tests/results/{}.csv", file_index)),
        file_index,
    );
}

#[allow(dead_code)]
fn surf_test() {
    let min_num_segments: usize = 3;
    let target_step: usize = 7;
    let x_size = 35;
    let target_num_segments = 4;

    let x_num_segments = target_num_segments.max(x_size / target_step).min(x_size - 1);

    let x_num_segments2 = (x_size / target_step).max(min_num_segments.min(x_size - 1));

    let linearly_space = |range: usize, num_segments: usize| -> Vec<usize> {
        let mut points: Vec<usize> = Vec::with_capacity(num_segments + 1);
        let step: f32 = (range as f32) / (num_segments as f32);
        let mut i: f32 = 0.0;
        loop {
            let j: usize = i.round() as usize;
            points.push(j);
            if j >= range {
                break;
            }
            i += step;
        }
        println!("Points: {:?}", points);
        points
    };
    println!("num segs: {}, {}", x_num_segments, x_num_segments2);

    // calculate initial points to search on surface
    let _is: Vec<usize> = linearly_space(x_size - 1, x_num_segments);
}

fn main() {
    let path = env::current_dir().expect("Could not locate current directory");
    println!("The current directory is {}", path.display());
    let _a = || {
        compare_result_folders(
            |file_index: usize| -> String { format!("tests/no_surface_optimisation/results_{}.csv", file_index) },
            |file_index: usize| -> String { format!("tests/surface_optimisation/results_{}.csv", file_index) },
            1..100
        );
    };
    run_simulations_using_input_file();
    //run_simulation_with_default_parameters();
    //surf_test();
}
