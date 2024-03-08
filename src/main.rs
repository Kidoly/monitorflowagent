use std::{io::Cursor, thread::sleep, time::Duration};
use sysinfo::{System, Disks, Networks, Components};
use xcap::{Monitor, image::RgbaImage};
use std::env;
use dotenv::dotenv;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok(); // Load .env file

    let api_key = env::var("API_KEY").expect("API_KEY not set in .env file");
    let api_url = env::var("API_URL").expect("API_URL not set in .env file");
    let interval: u64 = env::var("INTERVAL")
        .expect("INTERVAL not set in .env file")
        .parse()
        .expect("INTERVAL must be a valid number");

    loop {
        let mut sys = System::new_all();
        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();
        let components = Components::new_with_refreshed_list();
        let monitors = Monitor::all()?;
        sys.refresh_all();

	    if let Some(monitor) = monitors.first() {
            let data = generate_data(sys, disks, networks, components, monitor, &api_key, &interval);
            
            let req = ureq::request("POST", &api_url);
            println!("{:#?}", data);

            // Send a POST request
            let response =  req.set("Content-Type", "application/json")
                .send_string(&data.to_string()); 

            match response {
                Ok(response) => {
                    if response.status() == 200 {
                        println!("Data sent successfully");
                        let response_body = response.into_string().expect("Failed to read response body");
                        println!("Response: {}", response_body);
                    } else {
                        println!("Failed to send data: {}", response.status());
                        let error_body = response.into_string().expect("Failed to read response body");
                        println!("Error details: {}", error_body);
                    }
                },
                Err(e) => eprintln!("Failed to send data: {}", e),
            }
        } else {
            eprintln!("No monitors found");
        }
        // Wait for the next iteration
        sleep(Duration::from_secs(interval));
    }
}

fn generate_data(sys: System, disks: Disks, networks: Networks, components: Components, monitor: &Monitor, api_key: &str, interval: &u64) -> serde_json::Value {
    let avg_cpu_usage = sys
        .cpus()
        .iter()
        .map(|cpu| cpu.cpu_usage())
        .sum::<f32>() / sys.cpus().len() as f32;

    let monitor_image = match monitor.capture_image().map(|image| to_base64(image)) {
        Ok(image) => image,
        Err(e) => {
            eprintln!("Failed to capture monitor image: {}", e);
            String::new()
        }
    };

    let disks_json: Vec<serde_json::Value> = disks.iter()
    .map(|disk| {
        let name = disk.name();
        let mounted_on = disk.mount_point().display().to_string();
        serde_json::json!({
            "file_system": disk.file_system(),
            "total_space": disk.total_space(),
            "available_space": disk.available_space(),
        })
    })
    .collect();

    let processes_json: Vec<serde_json::Value> = sys.processes()
        .iter()
        .map(|(pid, process)| {
            serde_json::json!({
                "pid": pid.as_u32(),
                "name": process.name(),
                "start_time": process.start_time(),
                "cpu_usage": process.cpu_usage(),
                "memory": process.memory(),
            })
        })
        .collect();

    serde_json::json!({
        "api_key": api_key,
        "interval_time" : interval,
        "start_time": System::boot_time(),
        "total_memory": sys.total_memory().to_string(),
        "used_memory": sys.used_memory().to_string(),
        "total_swap": sys.total_swap().to_string(),
        "used_swap": sys.used_swap().to_string(),
        "system_name": System::name(),
        "kernel_version": System::kernel_version(),
        "os_version": System::os_version(),
        "host_name": System::host_name(),
        "cpu_count": sys.cpus().len(),
        "cpu_name": sys.cpus()[0].brand(),
        "cpu_usage": avg_cpu_usage,
        "disks_numbers": disks.len(),
        "disks": disks_json,
        "networks": networks.iter().map(|network|{format!("{network:#?}")}).collect::<Vec<_>>(),
        "processes_count": sys.processes().len(),
        "processes": processes_json,
        "monitor": monitor_image,
    })
}

fn to_base64(image: RgbaImage) -> String {
    let mut c = Cursor::new(Vec::new());
    image.write_to(&mut c, image::ImageOutputFormat::Png).unwrap();
    base64::encode(c.into_inner())
}

