use bincode::{Decode, Encode};
use std::{fs, net::{TcpListener, TcpStream}, thread, io::{Read, Write}, io};

#[derive(Encode, Decode, Debug, PartialEq)]
enum ReqType {
    Resolve,
    Register
}

#[derive(Encode, Decode, Debug)]
struct Package {
    reqtype: ReqType,
    payload: String
}

#[derive(Encode, Decode, Debug)]
struct Host {
    ip: Vec<u8>,
    addr: String,
}

fn main() -> io::Result<()> {
    // Crear directorio domains si no existe
    if !std::path::Path::new("./domains").exists() {
        match fs::create_dir("./domains") {
            Ok(_) => println!("Directorio 'domains' creado correctamente"),
            Err(e) => eprintln!("Error al crear directorio 'domains': {}", e)
        }
    }

    // Vincula el servidor a la dirección y puerto especificados
    let listener = TcpListener::bind("0.0.0.0:3000")?;
    println!("Servidor escuchando en 127.0.0.1:3000");

    // Acepta conexiones entrantes en un bucle
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("Nueva conexión establecida: {}", stream.peer_addr()?);
                // Maneja la conexión en un nuevo hilo
                thread::spawn(|| {
                    if let Err(e) = handle_client(stream) {
                        eprintln!("Error al manejar la conexión: {}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("Error al aceptar conexión: {}", e);
            }
        }
    }
    Ok(())
}

fn handle_client(mut stream: TcpStream) -> io::Result<()> {
    let mut buffer = [0; 512];
    loop {
        let bytes_read = stream.read(&mut buffer)?;
        if bytes_read == 0 {
            // Conexión cerrada por el cliente
            println!("Conexión cerrada por el cliente: {}", stream.peer_addr()?);
            break;
        }
        let data = &buffer[..bytes_read];
        let decoded_opt: Option<Package> = match bincode::decode_from_slice::<Package, _>(&data, bincode::config::standard()) {
            Ok((pkg, _)) => Some(pkg),
            Err(e) => {
                eprintln!("Error al deserializar el paquete: {}", e);
                None
            }
        };
        
        if let Some(decoded) = decoded_opt {
            match decoded.reqtype {
                ReqType::Resolve => {
                    println!("Resolviendo dominio: {}", decoded.payload.trim());
                    // Obtiene el Host o un Host vacío si no existe
                    let host_option = resolve(&decoded.payload.trim());
                    
                    // Esta es la parte importante: Extraer el Host del Option
                    // o crear un Host vacío si no existe
                    let host_to_send = match host_option {
                        Some(host) => host,
                        None => Host {
                            ip: Vec::new(),
                            addr: String::new()
                        }
                    };
                    
                    // Codifica el Host directamente (no el Option<Host>)
                    match bincode::encode_to_vec(&host_to_send, bincode::config::standard()) {
                        Ok(encoded) => {
                            stream.write_all(&encoded)?;
                            stream.flush()?;
                            println!("Respuesta enviada para {}", decoded.payload.trim());
                        },
                        Err(e) => {
                            eprintln!("Error al codificar respuesta: {}", e);
                        }
                    }
                },
                ReqType::Register => {
                    println!("Solicitud de registro recibida: {}", decoded.payload);
                    // Procesamiento del registro
                    // Formato esperado: dominio|ip
                    let parts: Vec<&str> = decoded.payload.split('|').collect();
                    if parts.len() == 2 {
                        let domain = parts[0].trim();
                        let ip_str = parts[1].trim();
                        let ip: Vec<u8> = ip_str.split('.')
                            .map(|octet| octet.parse::<u8>().unwrap_or(0))
                            .collect();
                        
                        let host = Host {
                            ip,
                            addr: domain.to_string()
                        };
                        
                        match register(host) {
                            Ok(_) => {
                                let response = "Dominio registrado con éxito";
                                stream.write_all(response.as_bytes())?;
                            },
                            Err(e) => {
                                let response = format!("Error al registrar dominio: {}", e);
                                stream.write_all(response.as_bytes())?;
                            }
                        }
                        stream.flush()?;
                    } else {
                        let response = "Formato inválido para registro. Debe ser 'dominio|ip'";
                        stream.write_all(response.as_bytes())?;
                        stream.flush()?;
                    }
                }
            }
        } else {
            let response = "Error al procesar la solicitud";
            stream.write_all(response.as_bytes())?;
            stream.flush()?;
        }
    }   
    Ok(())
}

fn register(host: Host) -> Result<(), Box<dyn std::error::Error>> {
    let file_path = format!("./domains/{}.sdm", host.addr);
    
    if std::path::Path::new(&file_path).exists() {
        println!("Dominio {} ya existe", host.addr);
        return Err(format!("El dominio {} ya existe", host.addr).into());
    } else {
        match bincode::encode_to_vec(&host, bincode::config::standard()) {
            Ok(encoded) => {
                match fs::write(&file_path, &encoded) {
                    Ok(()) => {
                        println!("Dominio {} registrado correctamente", host.addr);
                        Ok(())
                    },
                    Err(e) => {
                        let error_msg = format!("Error al escribir archivo de dominio: {}", e);
                        eprintln!("{}", error_msg);
                        Err(error_msg.into())
                    }
                }
            },
            Err(e) => {
                let error_msg = format!("Error al serializar datos del host: {}", e);
                eprintln!("{}", error_msg);
                Err(error_msg.into())
            }
        }
    }
}

fn resolve(domain: &str) -> Option<Host> {
    let file_path = format!("./domains/{}.sdm", domain);
    if std::path::Path::new(&file_path).exists() {
        match fs::read(&file_path) {
            Ok(file_content) => {
                match bincode::decode_from_slice::<Host, _>(&file_content, bincode::config::standard()) {
                    Ok((host, _)) => {
                        println!("Dominio {} resuelto correctamente", domain);
                        Some(host)
                    },
                    Err(e) => {
                        eprintln!("Error al deserializar el archivo '{}': {}", file_path, e);
                        None
                    }
                }
            },
            Err(e) => {
                eprintln!("Error al leer el archivo '{}': {}", file_path, e);
                None
            }
        }
    } else {
        println!("El dominio '{}' no existe", domain);
        None
    }
}