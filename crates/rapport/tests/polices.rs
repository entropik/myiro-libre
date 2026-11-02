//! Les polices embarquées sont des polices statiques bien nommées : famille
//! « Inter », sans trace de la police variable d'origine. Sinon le texte des
//! graphiques SVG ne trouve pas sa police et disparaît du PDF.

/// Les noms (identifiant, texte) d'une police TrueType, plate-forme Windows
/// en Unicode.
fn noms(police: &[u8]) -> Vec<(u16, String)> {
    let u16_ = |o: usize| u16::from_be_bytes([police[o], police[o + 1]]);
    let u32_ =
        |o: usize| u32::from_be_bytes([police[o], police[o + 1], police[o + 2], police[o + 3]]);
    let tables = u16_(4) as usize;
    let name = (0..tables)
        .map(|i| 12 + 16 * i)
        .find(|&e| &police[e..e + 4] == b"name")
        .map(|e| u32_(e + 8) as usize)
        .expect("table name");
    let (nombre, chaines) = (u16_(name + 2) as usize, name + u16_(name + 4) as usize);
    (0..nombre)
        .map(|i| name + 6 + 12 * i)
        .filter(|&r| u16_(r) == 3 && u16_(r + 2) == 1)
        .map(|r| {
            let (longueur, decalage) = (u16_(r + 8) as usize, u16_(r + 10) as usize);
            let octets = &police[chaines + decalage..chaines + decalage + longueur];
            let unites: Vec<u16> = octets
                .chunks(2)
                .map(|c| u16::from_be_bytes([c[0], c[1]]))
                .collect();
            (u16_(r + 6), String::from_utf16_lossy(&unites))
        })
        .collect()
}

#[test]
fn les_polices_sont_nommees_inter_sans_variable() {
    for (fichier, style) in [
        (
            &include_bytes!("../fontes/Inter-Regular.ttf")[..],
            "Regular",
        ),
        (
            &include_bytes!("../fontes/Inter-SemiBold.ttf")[..],
            "SemiBold",
        ),
    ] {
        let n = noms(fichier);
        let nom = |id| n.iter().find(|(i, _)| *i == id).map(|(_, t)| t.as_str());
        assert_eq!(nom(16), Some("Inter"), "{n:?}");
        assert_eq!(nom(17), Some(style), "{n:?}");
        assert!(n.iter().all(|(_, t)| !t.contains("Variable")), "{n:?}");
    }
}
