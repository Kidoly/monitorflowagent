use sysinfo::{System, Disks, Networks, Components};
use reqwest;
use tokio;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    loop {
        let mut sys = System::new_all();
        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();
        let components = Components::new_with_refreshed_list();
        sys.refresh_all();

        let data = generate_data(sys, disks, networks, components);
        println!("{:#?}", data);

    // Send a POST request
        match client.post("http://albanmary.com/api_receive.php")
            .json(&data)
            .send()
            .await {
                Ok(_) => println!("Data sent successfully"),
                Err(e) => eprintln!("Failed to send data: {}", e),
        }

        // Wait for 60 seconds before the next iteration
        sleep(Duration::from_secs(60)).await;
    }
}


fn generate_data(sys:System, disks:Disks, networks:Networks, components:Components) -> serde_json::Value {
    serde_json::json!({
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
    })
}
