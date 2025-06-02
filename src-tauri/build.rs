use std::process::Command;
use std::env;
use std::path::Path;

fn main() {
    // Собираем сервис только на Windows
    #[cfg(target_os = "windows")]
    build_service();

    // Запускаем tauri build после сборки сервиса
    tauri_build::build();
}

#[cfg(target_os = "windows")]
fn build_service() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let service_dir = format!("{}/../secure_link_windows_service", manifest_dir);

    // Проверяем, существует ли директория сервиса
    if !Path::new(&service_dir).exists() {
        println!("cargo:warning=Service directory not found: {}", service_dir);
        // Создаем пустой файл-заглушку для development
        let target_path = format!("{}/target/release/secure_link_windows_service.exe", manifest_dir);
        std::fs::create_dir_all(format!("{}/target/release", manifest_dir))
            .expect("Failed to create target directory");
        std::fs::write(&target_path, b"").expect("Failed to create placeholder file");
        return;
    }

    println!("cargo:rerun-if-changed={}/src", service_dir);
    println!("cargo:rerun-if-changed={}/Cargo.toml", service_dir);

    // Собираем сервис
    let output = Command::new("cargo")
        .args(&["build", "--release"])
        .current_dir(&service_dir)
        .output()
        .expect("Failed to execute cargo build for service");

    if !output.status.success() {
        panic!(
            "Failed to build service: {}\nstdout: {}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        );
    }

    // Определяем пути
    let service_exe_path = format!("{}/target/release/secure_link_windows_service.exe", service_dir);
    let target_dir = format!("{}/target/release", manifest_dir);
    let target_exe_path = format!("{}/secure_link_windows_service.exe", target_dir);

    // Создаем целевую директорию если она не существует
    std::fs::create_dir_all(&target_dir)
        .expect("Failed to create target directory");

    // Проверяем существование исходного файла
    if !Path::new(&service_exe_path).exists() {
        panic!("Service executable not found at: {}", service_exe_path);
    }

    // Копируем исполняемый файл
    if let Err(e) = std::fs::copy(&service_exe_path, &target_exe_path) {
        panic!("Failed to copy service executable from {} to {}: {}",
               service_exe_path, target_exe_path, e);
    }

    println!("cargo:warning=Successfully built and copied service executable");
}