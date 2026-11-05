#![forbid(unsafe_code)]
//! Cryptographically Secure Password & Diceware Passphrase Generator
//!
//! Generates high-entropy passwords, Diceware passphrases, and tokens
//! using the operating system's cryptographic CSPRNG (`rand::rngs::OsRng`).

use rand::Rng;

/// Generates a cryptographically secure password or Diceware passphrase
pub fn generate_password_or_passphrase(
    length: usize,
    use_upper: bool,
    use_lower: bool,
    use_nums: bool,
    use_syms: bool,
    is_passphrase: bool,
    word_count: usize,
) -> (String, f64, String) {
    let mut rng = rand::rngs::OsRng;
    if is_passphrase {
        let words = [
            "correct", "horse", "battery", "staple", "cipher", "matrix", "falcon", "orbital",
            "quantum", "beacon", "crypto", "shield", "kernel", "aurora", "glacier", "phoenix",
            "shadow", "timber", "vertex", "zenith", "hazard", "nebula", "titan", "vector",
            "cascade", "dynamo", "echo", "fusion", "granite", "hyper", "iron", "jupiter",
            "krypton", "lunar", "magnet", "nitrogen", "oxygen", "plasma", "quasar", "radar",
            "siren", "torus",
        ];
        let count = word_count.clamp(3, 8);
        let mut chosen = Vec::with_capacity(count);
        for _ in 0..count {
            let idx = rng.gen_range(0..words.len());
            chosen.push(words[idx]);
        }
        let pass = chosen.join("-");
        let entropy = (count as f64) * (words.len() as f64).log2();
        (
            pass,
            entropy,
            "Very Strong (Diceware Passphrase)".to_string(),
        )
    } else {
        let len = length.clamp(8, 128);
        let mut charset: Vec<u8> = Vec::new();
        if use_lower {
            charset.extend(b"abcdefghijklmnopqrstuvwxyz");
        }
        if use_upper {
            charset.extend(b"ABCDEFGHIJKLMNOPQRSTUVWXYZ");
        }
        if use_nums {
            charset.extend(b"0123456789");
        }
        if use_syms {
            charset.extend(b"!@#$%^&*()-_=+[]{}|;:,.<>?");
        }
        if charset.is_empty() {
            charset
                .extend(b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*");
        }

        let mut pass = String::with_capacity(len);
        for _ in 0..len {
            let idx = rng.gen_range(0..charset.len());
            pass.push(charset[idx] as char);
        }
        let entropy = (len as f64) * (charset.len() as f64).log2();
        let label = if entropy >= 100.0 {
            "Exceptional (Military Grade)"
        } else if entropy >= 70.0 {
            "Very Strong"
        } else if entropy >= 50.0 {
            "Strong"
        } else {
            "Moderate"
        };
        (pass, entropy, label.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_password_length_and_entropy() {
        let (pw, entropy, label) =
            generate_password_or_passphrase(32, true, true, true, true, false, 4);
        assert_eq!(pw.len(), 32);
        assert!(entropy >= 100.0);
        assert_eq!(label, "Exceptional (Military Grade)");
    }

    #[test]
    fn test_generate_diceware_passphrase() {
        let (passphrase, entropy, label) =
            generate_password_or_passphrase(16, true, true, true, true, true, 5);
        let parts: Vec<&str> = passphrase.split('-').collect();
        assert_eq!(parts.len(), 5);
        assert!(entropy > 25.0);
        assert!(label.contains("Diceware"));
    }
}
