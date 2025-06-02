use std::process::Command;
use std::env;
use std::path::Path;

fn main() {
    
    // Собираем сервис только на Windows
    #[cfg(target_os = "windows")]
    build_service();
    
    // Встраиваем манифест на Windows
    #[cfg(target_os = "windows")]
    build_tauri_with_embed_admin_manifest();

    #[cfg(not(target_os = "windows"))]
    {
        tauri_build::build();
    }

}

#[cfg(target_os = "windows")]
fn build_tauri_with_embed_admin_manifest() {
    
    let mut windows = tauri_build::WindowsAttributes::new();

    let mut windows = tauri_build::WindowsAttributes::new();
    windows = windows.app_manifest(
        r#"<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency>
    <dependentAssembly>
      <assemblyIdentity
        type="win32"
        name="Microsoft.Windows.Common-Controls"
        version="6.0.0.0"
        processorArchitecture="*"
        publicKeyToken="6595b64144ccf1df"
        language="*"
      />
    </dependentAssembly>
  </dependency>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
        <requestedPrivileges>
            <requestedExecutionLevel level="requireAdministrator" uiAccess="false" />
        </requestedPrivileges>
    </security>
  </trustInfo>
</assembly>
"#,
    );

    tauri_build::try_build(
        tauri_build::Attributes::new().windows_attributes(windows)
    ).expect("failed to run build script");
    
}

#[cfg(target_os = "windows")]
fn build_service() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let service_dir = format!("{}/../secure_link_windows_service", manifest_dir);

    println!("cargo:rerun-if-changed={}/src", service_dir);
    println!("cargo:rerun-if-changed={}/Cargo.toml", service_dir);

    // Создаем команду
    let mut cmd = Command::new("cargo");
    cmd.args(&["build", "--release"])
        .current_dir(&service_dir);

    // Добавляем target только если он установлен
    let target_opt = if let Ok(target) = env::var("TARGET") {
        println!("cargo:warning=Building service for target: {}", target);
        cmd.arg("--target").arg(&target);
        Some(target)
    } else {
        println!("cargo:warning=Building service for default target");
        None
    };

    // Собираем сервис
    let output = cmd.output()
        .expect("Failed to execute cargo build for service");

    if !output.status.success() {
        panic!(
            "Failed to build service: {}\nstdout: {}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        );
    }

    // Определяем пути с учетом target
    let service_exe_path = 
        if let Some(target) = target_opt {
            format!("{}/target/{}/release/secure_link_windows_service.exe", service_dir, target)
        } else {
            format!("{}/target/release/secure_link_windows_service.exe", service_dir)
        };
    
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