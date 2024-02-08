use std::io::Cursor;

use sysinfo::{System, Disks, Networks, Components};
use tokio::time::{sleep, Duration};
use xcap::{Monitor, image::{RgbaImage}};
use std::env;
use dotenv::dotenv;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok(); // Load .env file

    let password = env::var("PASSWORD").expect("PASSWORD not set in .env file");
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
            let data = generate_data(sys, disks, networks, components, monitor, &password); // Pass password here
            let client = reqwest::Client::new();
            println!("{:#?}", data);

            // Send a POST request
            let response = client.post(&api_url)
            .json(&data)
            .send()
            .await;
        
            match response {
                Ok(response) => {
                    if response.status().is_success() {
                        println!("Data sent successfully");
                        let response_body = response.text().await?;
                        println!("Response: {}", response_body);
                    } else {
                        println!("Failed to send data: {}", response.status());
                        let error_body = response.text().await?;
                        println!("Error details: {}", error_body);
                    }
                },
                Err(e) => eprintln!("Failed to send data: {}", e),
            }
        } else {
            eprintln!("No monitors found");
        }
        // Wait for the next iteration
        sleep(Duration::from_secs(interval)).await;
    }
}

fn generate_data(sys: System, disks: Disks, networks: Networks, components: Components, monitor: &Monitor, password: &str) -> serde_json::Value {
    let monitor_image = match monitor.capture_image().map(|image| to_base64(image)) {
        Ok(image) => image,
        Err(e) => {
            eprintln!("Failed to capture monitor image: {}", e);
            String::new()
        }
    };
    serde_json::json!({
        "password": password,
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
        "disks_numbers": disks.len(),
        "disks": disks.iter().map(|disk|{format!("{disk:#?}")}).collect::<Vec<_>>(),
        "networks": networks.iter().map(|network|{format!("{network:#?}")}).collect::<Vec<_>>(),
        "components": components.iter().map(|component|{format!("{component:#?}")}).collect::<Vec<_>>(),
        "processes_count": sys.processes().len(),
        "processes": sys.processes().iter().map(|process|{format!("{process:#?}")}).collect::<Vec<_>>(),
        "monitor": monitor_image,
    })
}

fn to_base64(image: RgbaImage) -> String {
    let mut c = Cursor::new(Vec::new());
    image.write_to(&mut c, image::ImageOutputFormat::Png).unwrap();
    base64::encode(c.into_inner())
}

