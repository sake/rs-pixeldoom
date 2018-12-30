extern crate image;

use image::GenericImageView;
use std::env;
use std::io::{self, Write};
use std::thread;
use std::net::TcpStream;


struct PixelConnection {
    offset: (u32, u32),
    addr: String,
    stream: Vec<TcpStream>,
    numpkts: u32,
}

impl PixelConnection {
    fn create(addr: String) -> Result<PixelConnection, io::Error> {
        let mut inst = PixelConnection {
            offset: (0, 0),//(1619,751),
            addr: addr,
            stream: Vec::new(),
            numpkts: 0,
        };

        let conn = inst.connect()?;
        inst.stream.push(conn);

        return Ok(inst);
    }

    fn send_pixel(&mut self, x: u32, y: u32, p: &image::Rgba<u8>) -> Result<(), io::Error> {
        let [r, g, b, a] = p.data;
        let ox = self.offset.0 + x; 
        let oy = self.offset.1 + y; 
        if a > 0 {
            let line: String = format!("PX {} {} {:02x?}{:02x?}{:02x?}{:02x?}\n", ox, oy, r, g, b, a);
            let mut bytes = line.as_bytes();
            loop {
                match self.stream
                    .get(0)
                    .expect("Uninitialized connection used.")
                    .write(bytes) {
                        Ok(res) => {
                            if res < bytes.len() {
                                bytes = bytes.split_at(res).1;
                            } else {
                                return Ok(());
                            }
                        },
                        Err(err) => {
                            if err.kind() == io::ErrorKind::WouldBlock {
                                // just try again
                                //println!("Would block hit.");
                                let sleep_time = std::time::Duration::from_millis(10);
                                std::thread::sleep(sleep_time);
                            } else {
                                return Err(err);
                            }
                        }
                    }
            }
            // increment pkt count
            self.numpkts += 1;

            // if self.numpkts > 3000 {
            //     println!("Flushing stream.");
            //     self.stream
            //     .get(0)
            //     .expect("Uninitialized connection used.")
            //     .flush()?;
            //     self.numpkts = 0;
            // }

            return Ok(());
        } else {
            return Ok(());
        }
    }

    fn connect(&self) -> Result<TcpStream, io::Error> {
        println!("Trying to connect to host {} ...", self.addr);
        let stream = TcpStream::connect(&self.addr)?;
        //stream.set_nodelay(true)?;
        stream.set_nonblocking(true)?;

        return Ok(stream);
    }

    fn reconnect(&mut self) {
        loop {
            println!("Trying to reconnect socket.");
            self.stream.clear();
            match self.connect() {
                Ok(c) => {
                    self.stream.push(c);
                    return ();
                }
                Err(e) => {
                    println!("{:?}", e);
                    // now try again
                }
            }
        }
    }
}

fn main() {
    // read args
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        println!("USAGE:");
        println!("  {} <host:port> <imagefile>", &args[0]);
        std::process::exit(1);
    }

    let addr = &args[1];
    let img_file = &args[2];

    let img = image::open(&img_file).unwrap();
    println!("Loaded image with size {}x{}.", img.width(), img.height());

    let num_threads = 1;
    for i in 0..num_threads {
        println!("Spawning thread number {}.", i+1);

        // get view to image
        let img_new: image::DynamicImage = img.clone();
        let stripe_width = img.width() / num_threads;
        let stripe_height = img.height();
        let stripe_x = i * stripe_width;
        //img_new.copy_from(img, 0, 0);
        
        // open connection
        let mut con = PixelConnection::create(addr.to_string()).expect("Initial connection failed.");

        thread::spawn(move || {
            let img_view = img_new.view(stripe_x, 0, stripe_width, stripe_height);
            loop {
                for (x, y, p) in img_view.pixels() {
                    match con.send_pixel(x, y, &p) {
                        Ok(_) => (), // ignore
                        Err(_e) => 
                            con.reconnect(),
                    }
                }
            }
        });
    }

    thread::park();
}
