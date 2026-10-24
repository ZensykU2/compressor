mod bits;
use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Benutzung:");
        eprintln!(" kompressor entropy <datei>");
        eprintln!(" kompressor compress <eingabe> <ausgabe>");
        eprintln!(" kompressor decompress <eingabe> <ausgabe>");
        process::exit(1);
    }

    match args[1].as_str() {
        "entropy" => {
            let data = fs::read(&args[2]).expect("Datei konnte nicht gelesen werden");
            let h = order0_entropy(&data);
            let min_bytes = h * data.len() as f64 / 8.0;
            println!("Grösse:   {} Bytes", data.len());
            println!("entropy: {:.4} Bits pro Byte", h);
            println!("Theor. Min.:  {:.0} Bytes", min_bytes);
        }
        "compress" | "decompress" => {
            if args.len() < 4 {
                eprintln!("Ausgabedatei fehlt");
                process::exit(1);
            }
            let data = fs::read(&args[2]).expect("Eingabe nicht lesbar");
            fs::write(&args[3], &data).expect("Ausgabe nicht schreibbar");
        }
        other => {
            eprintln!("Unbekannter Befehl: {other}");
            process::exit(1);
        }
    }
}

fn order0_entropy(data: &[u8]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }

    let mut counter = [0u64; 256];
    for &byte in data {
        counter[byte as usize] += 1;
    }

    let n = data.len() as f64;
    let mut entropy = 0.0;
    for &count in counter.iter() {
        if count > 0 {
            let p = count as f64 / n;
            entropy -= p * p.log2();
        }
    }
    entropy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nur_ein_wert_hat_n() {
        assert_eq!(order0_entropy(&[7, 7, 7, 7]), 0.0);
    }

    #[test]
    fn zwei_werte_gleich_oft_ergibt_ein_bit() {
        assert_eq!(order0_entropy(&[0, 1, 0, 1]), 1.0);
    }
}
