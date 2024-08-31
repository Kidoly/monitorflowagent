use dotenvy::dotenv;
use std::{
    env,
    fs::File,
    io::{prelude::*, Cursor},
    thread::sleep,
    time::Duration,
};
use sysinfo::{Components, Disks, Networks, System};
use xcap::{image::RgbaImage, Monitor};
use base64::{engine::general_purpose, Engine};



fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().expect(".env file not found");

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

fn generate_data(sys: System, disks: Disks, networks: Networks, _components: Components, monitor: &Monitor, api_key: &str, interval: &u64) -> serde_json::Value {
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
        let _name = disk.name();
        let _mounted_on = disk.mount_point().display().to_string();
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
    general_purpose::STANDARD.encode(c.into_inner())
}

//Read the uuid from the `info` file in the current directory.
fn read_uuid() -> std::io::Result<uuid::Uuid> {
    let mut file = File::open("info")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let uuid = contents.split("-----BEGIN UUID-----\n").collect::<Vec<&str>>()[1].split("\n-----END UUID-----").collect::<Vec<&str>>()[0];
    Ok(uuid::Uuid::parse_str(uuid).unwrap())
}

//Read the services to verify from the `info` file in the current directory.
fn read_services_to_verify() -> std::io::Result<Vec<String>> {
    let mut file = File::open("info")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let services = contents.split("-----BEGIN SERVICES TO VERIFY-----\n").collect::<Vec<&str>>()[1].split("\n-----END SERVICES TO VERIFY-----").collect::<Vec<&str>>()[0];
    Ok(services.split("\n").map(|s| s.to_string()).collect())
}

//Add a service to the `info` file in the current directory if it is not already there.
fn add_service_to_verify(service: &str) -> std::io::Result<()> {
    let mut file = File::open("info")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let services = contents.split("-----BEGIN SERVICES TO VERIFY-----\n").collect::<Vec<&str>>()[1].split("\n-----END SERVICES TO VERIFY-----").collect::<Vec<&str>>()[0].to_string();
    if !services.contains(service) {
        let new_services = format!("{}\n{}", services, service);
        let new_contents = contents.replace(&contents.split("-----BEGIN SERVICES TO VERIFY-----\n").collect::<Vec<&str>>()[1].split("\n-----END SERVICES TO VERIFY-----").collect::<Vec<&str>>()[0], &new_services);
        let mut file = File::create("info").unwrap();
        file.write_all(new_contents.as_bytes()).unwrap();
    }
    Ok(())
}

//Remove a service from the `info` file in the current directory.
fn remove_service_to_verify(service: &str) -> std::io::Result<()> {
    let mut file = File::open("info")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let services = contents.split("-----BEGIN SERVICES TO VERIFY-----\n").collect::<Vec<&str>>()[1].split("\n-----END SERVICES TO VERIFY-----").collect::<Vec<&str>>()[0].to_string();
    if services.contains(service) {
        let new_services = services.replace(service, "");
        let new_contents = contents.replace("\n\n", "\n");
        let new_contents = contents.replace(&contents.split("-----BEGIN SERVICES TO VERIFY-----\n").collect::<Vec<&str>>()[1].split("\n-----END SERVICES TO VERIFY-----").collect::<Vec<&str>>()[0], &new_services);
        let mut file = File::create("info").unwrap();
        file.write_all(new_contents.as_bytes()).unwrap();
    }
    Ok(())
}

//Delete empty lines from the `info` file in the current directory.
fn delete_empty_lines() -> std::io::Result<()> {
    let mut file = File::open("info")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let new_contents = contents.replace("\n\n", "\n");
    let mut file = File::create("info").unwrap();
    file.write_all(new_contents.as_bytes()).unwrap();
    Ok(())
}

//Read the tasks to verify from the `info` file in the current directory.
fn read_tasks_to_verify() -> std::io::Result<Vec<String>> {
    let mut file = File::open("info")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let tasks = contents.split("-----BEGIN TASKS TO VERIFY-----\n").collect::<Vec<&str>>()[1].split("\n-----END TASKS TO VERIFY-----").collect::<Vec<&str>>()[0];
    Ok(tasks.split("\n").map(|s| s.to_string()).collect())
}

//Add a task to the `info` file in the current directory if it is not already there.
fn add_task_to_verify(task: &str) -> std::io::Result<()> {
    let mut file = File::open("info")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let tasks = contents.split("-----BEGIN TASKS TO VERIFY-----\n").collect::<Vec<&str>>()[1].split("\n-----END TASKS TO VERIFY-----").collect::<Vec<&str>>()[0].to_string();
    if !tasks.contains(task) {
        let new_tasks = format!("{}\n{}", tasks, task);
        let new_contents = contents.replace(&contents.split("-----BEGIN TASKS TO VERIFY-----\n").collect::<Vec<&str>>()[1].split("\n-----END TASKS TO VERIFY-----").collect::<Vec<&str>>()[0], &new_tasks);
        let mut file = File::create("info").unwrap();
        file.write_all(new_contents.as_bytes()).unwrap();
    }
    Ok(())
}

//Remove a task from the `info` file in the current directory.
fn remove_task_to_verify(task: &str) -> std::io::Result<()> {
    let mut file = File::open("info")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let tasks = contents.split("-----BEGIN TASKS TO VERIFY-----\n").collect::<Vec<&str>>()[1].split("\n-----END TASKS TO VERIFY-----").collect::<Vec<&str>>()[0].to_string();
    if tasks.contains(task) {
        let new_tasks = tasks.replace(task, "");
        let new_contents = contents.replace("\n\n", "\n");
        let new_contents = contents.replace(&contents.split("-----BEGIN TASKS TO VERIFY-----\n").collect::<Vec<&str>>()[1].split("\n-----END TASKS TO VERIFY-----").collect::<Vec<&str>>()[0], &new_tasks);
        let mut file = File::create("info").unwrap();
        file.write_all(new_contents.as_bytes()).unwrap();
    }
    Ok(())
}

//Create a file called `info` in the current directory that looks like this:
//-----BEGIN UUID-----
//[UUID]
//-----END UUID-----
//-----BEGIN SERVICES TO VERIFY-----
//
//-----END SERVICES TO VERIFY-----
//-----BEGIN TASKS TO VERIFY-----
//
//-----END TASKS TO VERIFY-----
//Where [UUID] is a randomly generated UUID. You can use the `uuid` crate to generate a UUID.
fn create_info_file() -> std::io::Result<()> {
    let mut file = File::create("info").unwrap();
    let uuid = uuid::Uuid::new_v4();
    file.write_all(format!("-----BEGIN UUID-----\n{}\n-----END UUID-----\n-----BEGIN SERVICES TO VERIFY-----\n\n-----END SERVICES TO VERIFY-----\n-----BEGIN TASKS TO VERIFY-----\n\n-----END TASKS TO VERIFY-----", uuid).as_bytes()).unwrap();
    Ok(())
}

//Verify there is file called `info` in the current directory.
fn test_info_file() -> std::io::Result<()> {
    File::open("info")?;
    Ok(())
}
