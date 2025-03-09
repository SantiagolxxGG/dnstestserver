use bincode::{Decode, Encode};
use console::style;
use crossterm;
use dialoguer::Input;
use indicatif::{ProgressBar, ProgressStyle};
use std::{
    io::{self, Read, Write, stdout},
    net::TcpStream,
    thread::sleep,
    time::Duration,
};

#[derive(bincode::Encode, Debug)]
enum ReqType {
    Resolve,
    Register,
}

#[derive(bincode::Encode, Debug)]
struct Package {
    reqtype: ReqType,
    payload: String,
}

#[derive(Encode, Decode, Debug)]
struct Host {
    ip: Vec<u8>,
    addr: String,
}
use dialoguer::{Select, theme::ColorfulTheme};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opciones = vec!["Resolver dominio", "Registrar dominio", "Salir"];

    // Crea el menú de selección
    let seleccion = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Por favor, selecciona una opción:")
        .default(0)
        .items(&opciones)
        .interact()
        .unwrap();

    // Maneja la selección del usuario
    match seleccion {
        0 => {
            let spinner = ProgressBar::new_spinner();
            spinner.set_style(
                ProgressStyle::default_spinner()
                    .tick_strings(&["◐", "◓", "◑", "◒", "✔"])
                    .template("{spinner:.green} {msg}")
                    .unwrap(),
            );

            spinner.enable_steady_tick(Duration::from_millis(100));
            spinner.set_message("Intentando conexion al servidor DNS...");

            match std::net::TcpStream::connect("127.0.0.1:3000") {
                Ok(mut s) => {
                    sleep(Duration::from_millis(500));
                    spinner.finish_with_message("Conectado al servidor DNS");
                    let name: String = Input::new()
                        .with_prompt("Ingresa el dominio al cual resolver")
                        .interact_text()?;

                    let req = Package {
                        reqtype: ReqType::Resolve,
                        payload: name,
                    };
                    success_print("Request ensamblado correctamente");
                    let enconded: Option<Vec<u8>> =
                        match bincode::encode_to_vec(&req, bincode::config::standard()) {
                            Ok(e) => Some(e),
                            Err(e) => {
                                eprintln!("Error al serializar datos de registro a binario: {}", e);
                                None
                            }
                        };
                    success_print("Request codificado correctamente");

                    s.write_all(&enconded.unwrap());
                    success_print("Request enviado al servidor");
                    let spinner2 = ProgressBar::new_spinner();
                    spinner2.set_style(
                        ProgressStyle::with_template("{spinner:.blue} {msg}")
                            .unwrap()
                            .tick_strings(&["◐", "◓", "◑", "◒", "✔"]),
                    );
                    spinner2.enable_steady_tick(Duration::from_millis(100));
                    spinner2.set_message("Esperando respuesta");
                    sleep(Duration::from_secs(1));

                    let mut buffer = [0; 1024];
                    let bytes_read = s.read(&mut buffer)?;
                    let data = &buffer[..bytes_read];
                    spinner2.finish_with_message("Respuesta recibida");

                    // Ahora esperamos recibir un Host directamente, no un Option<Host>
                    match bincode::decode_from_slice::<Host, _>(&data, bincode::config::standard()) {
                        Ok((host, _)) => {
                            if !host.addr.is_empty() {
                                success_print("Dominio resuelto correctamente");
                                println!("IP: {}", host.ip.iter()
                                    .map(|octet| octet.to_string())
                                    .collect::<Vec<String>>()
                                    .join("."));
                                println!("Dirección: {}", host.addr);
                            } else {
                                error_print("El dominio no existe o no pudo ser resuelto");
                            }
                        },
                        Err(e) => {
                            error_print(&format!("Error al decodificar la respuesta: {}", e));
                        }
                    }
                }
                Err(e) => {
                    error_print(&format!("Error al conectarse al servidor DNS: {}", e));
                }
            }
        }
        1 => {
            let spinner = ProgressBar::new_spinner();
            spinner.set_style(
                ProgressStyle::default_spinner()
                    .tick_strings(&["◐", "◓", "◑", "◒", "✔"])
                    .template("{spinner:.green} {msg}")
                    .unwrap(),
            );

            spinner.enable_steady_tick(Duration::from_millis(100));
            spinner.set_message("Intentando conexion al servidor DNS...");

            match std::net::TcpStream::connect("127.0.0.1:3000") {
                Ok(mut s) => {
                    sleep(Duration::from_millis(500));
                    spinner.finish_with_message("Conectado al servidor DNS");
                    
                    let domain: String = Input::new()
                        .with_prompt("Ingresa el nombre de dominio a registrar")
                        .interact_text()?;
                    
                    let ip_str: String = Input::new()
                        .with_prompt("Ingresa la dirección IP (formato: 192.168.1.1)")
                        .interact_text()?;
                    
                    // Validación básica de formato de IP
                    let parts: Vec<&str> = ip_str.split('.').collect();
                    if parts.len() != 4 || parts.iter().any(|part| part.parse::<u8>().is_err()) {
                        error_print("Formato de IP inválido. Debe ser IPv4 (ejemplo: 192.168.1.1)");
                        return Ok(());
                    }
                    
                    let req = Package {
                        reqtype: ReqType::Register,
                        payload: format!("{}|{}", domain, ip_str),
                    };
                    
                    success_print("Request de registro ensamblado correctamente");
                    let encoded: Option<Vec<u8>> =
                        match bincode::encode_to_vec(&req, bincode::config::standard()) {
                            Ok(e) => Some(e),
                            Err(e) => {
                                eprintln!("Error al serializar datos de registro a binario: {}", e);
                                None
                            }
                        };
                    
                    success_print("Request codificado correctamente");
                    s.write_all(&encoded.unwrap());
                    success_print("Request de registro enviado al servidor");
                    
                    let spinner2 = ProgressBar::new_spinner();
                    spinner2.set_style(
                        ProgressStyle::with_template("{spinner:.blue} {msg}")
                            .unwrap()
                            .tick_strings(&["◐", "◓", "◑", "◒", "✔"]),
                    );
                    spinner2.enable_steady_tick(Duration::from_millis(100));
                    spinner2.set_message("Esperando respuesta");
                    sleep(Duration::from_secs(1));
                    
                    let mut buffer = [0; 1024];
                    let bytes_read = s.read(&mut buffer)?;
                    
                    if bytes_read > 0 {
                        let data = &buffer[..bytes_read];
                        spinner2.finish_with_message("Respuesta recibida");
                        
                        // Mostrar la respuesta del servidor como texto
                        match std::str::from_utf8(data) {
                            Ok(response) => {
                                if response.contains("éxito") || response.contains("exitosamente") || response.contains("correctamente") {
                                    success_print(response);
                                } else {
                                    error_print(response);
                                }
                            },
                            Err(_) => {
                                error_print("Respuesta del servidor no pudo ser interpretada");
                            }
                        }
                    } else {
                        spinner2.finish_with_message("No se recibió respuesta");
                        error_print("El servidor no envió respuesta");
                    }
                }
                Err(e) => {
                    error_print(&format!("Error al conectarse al servidor DNS: {}", e));
                }
            }
        }
        2 => {
            println!("Saliendo...");
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn success_print(msg: &str) {
    let check_mark = style("✔").green();
    let message = style(msg).blue();
    println!("{} {}", check_mark, message);
}

fn error_print(msg: &str) {
    let check_mark = style("✘").red();
    let message = style(msg).red();
    println!("{} {}", check_mark, message);
}