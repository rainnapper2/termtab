const SHARPS: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
const FLATS: [&str; 12] = ["C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B"];

pub fn parse_key_signature(annotation: &str) -> Option<String> {
    let lower = annotation.trim().to_lowercase();
    if lower.starts_with("key:") {
        let key = lower.trim_start_matches("key:").trim().to_string();
        Some(key)
    } else {
        None
    }
}

pub fn uses_flats(key: &str) -> bool {
    let k = key.to_lowercase();
    // Common flat keys
    k.contains("f major") || 
    k.contains("bb") || 
    k.contains("eb") || 
    k.contains("ab") || 
    k.contains("db") || 
    k.contains("gb") || 
    k.contains("d minor") || 
    k.contains("g minor") || 
    k.contains("c minor") || 
    k.contains("f minor")
}

pub fn get_base_note_index(tuning: char) -> Option<usize> {
    match tuning {
        'C' | 'c' => Some(0),
        'D' | 'd' => Some(2),
        'E' | 'e' => Some(4),
        'F' | 'f' => Some(5),
        'G' | 'g' => Some(7),
        'A' | 'a' => Some(9),
        'B' | 'b' => Some(11),
        _ => None,
    }
}

pub fn fret_to_note(tuning: char, fret: u32, key_signature: Option<&str>) -> String {
    let base = get_base_note_index(tuning).unwrap_or(4); // Default to E if unknown
    let index = (base + fret as usize) % 12;

    let use_flats = match key_signature {
        Some(k) => uses_flats(k),
        None => false, // Default to sharps
    };

    if use_flats {
        FLATS[index].to_string()
    } else {
        SHARPS[index].to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fret_to_note() {
        assert_eq!(fret_to_note('E', 8, None), "C"); // 8th fret on E is C
        assert_eq!(fret_to_note('E', 0, None), "E"); // open E is E
        assert_eq!(fret_to_note('E', 12, None), "E"); // 12th fret E is E
        assert_eq!(fret_to_note('e', 8, None), "C"); // 8th fret on high e is C
        
        // Key signatures
        assert_eq!(fret_to_note('E', 9, Some("Key: C Major")), "C#");
        assert_eq!(fret_to_note('E', 9, Some("Key: F Major")), "Db");
        assert_eq!(fret_to_note('A', 1, Some("Key: Bb Minor")), "Bb");
        assert_eq!(fret_to_note('G', 3, Some("Key: Bb Minor")), "Bb");
    }

    #[test]
    fn test_key_signature_parser() {
        assert_eq!(parse_key_signature("Key: Bb Minor").unwrap(), "bb minor");
        assert_eq!(parse_key_signature("   Key: C Major   ").unwrap(), "c major");
        assert_eq!(parse_key_signature("something else"), None);
    }
}
