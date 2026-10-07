//! Catalogue des textes de l'interface, en français et en anglais.

/// Langue d'une fenêtre : une seule à la fois.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Langue {
    Francais,
    Anglais,
}

impl Langue {
    /// Code court de la langue : `fr` ou `en`.
    pub fn code(self) -> &'static str {
        match self {
            Langue::Francais => "fr",
            Langue::Anglais => "en",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Langue> {
        match code {
            "fr" => Some(Langue::Francais),
            "en" => Some(Langue::Anglais),
            _ => None,
        }
    }
}

/// Toutes les clés du catalogue.
pub fn cles() -> impl Iterator<Item = &'static str> {
    FRANCAIS.iter().map(|(c, _)| *c)
}

type Catalogue = &'static [(&'static str, &'static str)];

// Typographie française : apostrophe courbe (’), espace fine insécable
// (\u{202f}) avant `: ; ! ?` et entre un nombre et son unité.
const FRANCAIS: Catalogue = &[
    // Barre du haut
    ("nav.libelle", "Tâches"),
    ("tache.mesurer", "Mesurer"),
    ("tache.controler", "Contrôler"),
    ("tache.profiler", "Profiler"),
    ("tache.bibliotheque", "Bibliothèque"),
    ("instrument.libelle", "Instrument"),
    ("instrument.aucun", "Aucun instrument"),
    ("theme.libelle", "Thème"),
    ("theme.clair", "Clair"),
    ("theme.sombre", "Sombre"),
    ("langue.libelle", "Langue"),
    // Bibliothèque, à gauche
    ("bibliotheque.titre", "Conditions d’impression"),
    ("bibliotheque.exemple", "Exemple, données fictives"),
    ("exemple.condition1", "Jet d’encre, brillant 260\u{202f}g"),
    ("exemple.condition2", "Offset, couché mat 150\u{202f}g"),
    ("bibliotheque.mires", "Mires"),
    ("bibliotheque.mesures", "Mesures"),
    ("bibliotheque.profils", "Profils"),
    ("bibliotheque.controles", "Contrôles"),
    // Feuille de travail, au centre
    ("feuille.numero", "Feuille 01"),
    ("vide.mesurer.phrase", "Aucune mesure."),
    ("vide.mesurer.action", "Mesurer une couleur"),
    ("vide.controler.phrase", "Aucun contrôle d’impression."),
    ("vide.controler.action", "Contrôler un tirage"),
    ("vide.profiler.phrase", "Aucun profil."),
    ("vide.profiler.action", "Créer un profil"),
    ("vide.bibliotheque.phrase", "Aucune condition d’impression."),
    (
        "vide.bibliotheque.action",
        "Ajouter une condition d’impression",
    ),
    (
        "raison.instrument",
        "Disponible quand un instrument est branché.",
    ),
    ("raison.bientot", "Disponible dans une prochaine version."),
    // Détails, à droite
    ("details.titre", "Détails"),
    ("cartouche.libelle", "Provenance"),
    ("cartouche.titre", "Aucun résultat"),
    ("cartouche.instrument", "Instrument"),
    ("cartouche.etalonnage", "Étalonnage"),
    ("cartouche.condition_mesure", "Condition de mesure"),
    ("cartouche.reference", "Référence"),
    ("cartouche.aucun", "Aucun"),
    ("cartouche.aucune", "Aucune"),
    ("cartouche.non_fait", "Non fait"),
];

const ANGLAIS: Catalogue = &[
    // Top bar
    ("nav.libelle", "Tasks"),
    ("tache.mesurer", "Measure"),
    ("tache.controler", "Check"),
    ("tache.profiler", "Profile"),
    ("tache.bibliotheque", "Library"),
    ("instrument.libelle", "Instrument"),
    ("instrument.aucun", "No instrument"),
    ("theme.libelle", "Theme"),
    ("theme.clair", "Light"),
    ("theme.sombre", "Dark"),
    ("langue.libelle", "Language"),
    // Library, left
    ("bibliotheque.titre", "Printing conditions"),
    ("bibliotheque.exemple", "Example, sample data"),
    ("exemple.condition1", "Inkjet, glossy 260\u{a0}g"),
    ("exemple.condition2", "Offset, matte coated 150\u{a0}g"),
    ("bibliotheque.mires", "Charts"),
    ("bibliotheque.mesures", "Measurements"),
    ("bibliotheque.profils", "Profiles"),
    ("bibliotheque.controles", "Checks"),
    // Work sheet, centre
    ("feuille.numero", "Sheet 01"),
    ("vide.mesurer.phrase", "No measurement yet."),
    ("vide.mesurer.action", "Measure a colour"),
    ("vide.controler.phrase", "No print check yet."),
    ("vide.controler.action", "Check a print"),
    ("vide.profiler.phrase", "No profile yet."),
    ("vide.profiler.action", "Create a profile"),
    ("vide.bibliotheque.phrase", "No printing condition yet."),
    ("vide.bibliotheque.action", "Add a printing condition"),
    (
        "raison.instrument",
        "Available once an instrument is connected.",
    ),
    ("raison.bientot", "Available in a future version."),
    // Details, right
    ("details.titre", "Details"),
    ("cartouche.libelle", "Provenance"),
    ("cartouche.titre", "No result"),
    ("cartouche.instrument", "Instrument"),
    ("cartouche.etalonnage", "Calibration"),
    ("cartouche.condition_mesure", "Measurement condition"),
    ("cartouche.reference", "Reference"),
    ("cartouche.aucun", "None"),
    ("cartouche.aucune", "None"),
    ("cartouche.non_fait", "Not done"),
];

fn catalogue(langue: Langue) -> Catalogue {
    match langue {
        Langue::Francais => FRANCAIS,
        Langue::Anglais => ANGLAIS,
    }
}

/// Texte de l'interface pour une clé, dans une langue. Une clé inconnue est
/// rendue telle quelle, pour rester visible à l'écran.
pub fn texte(langue: Langue, cle: &str) -> &str {
    catalogue(langue)
        .iter()
        .find(|(c, _)| *c == cle)
        .map_or(cle, |(_, t)| t)
}

/// Clé présente dans une langue et absente de l'autre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleManquante {
    pub cle: &'static str,
    /// Langue où la clé manque.
    pub absente_en: Langue,
}

/// Clés qui manquent dans l'une des deux langues. Vide quand les catalogues
/// français et anglais sont complets.
pub fn cles_manquantes() -> Vec<CleManquante> {
    let absentes = |de: Catalogue, dans: Catalogue, absente_en: Langue| {
        de.iter()
            .filter(move |(c, _)| !dans.iter().any(|(d, _)| d == c))
            .map(move |(cle, _)| CleManquante { cle, absente_en })
    };
    absentes(FRANCAIS, ANGLAIS, Langue::Anglais)
        .chain(absentes(ANGLAIS, FRANCAIS, Langue::Francais))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_taches_ont_leur_nom_dans_chaque_langue() {
        assert_eq!(texte(Langue::Francais, "tache.mesurer"), "Mesurer");
        assert_eq!(texte(Langue::Anglais, "tache.mesurer"), "Measure");
    }

    #[test]
    fn chaque_cle_existe_en_francais_et_en_anglais() {
        assert_eq!(cles_manquantes(), vec![]);
    }

    /// Typographie française : apostrophe courbe, guillemets français, espace
    /// fine insécable (U+202F) avant `: ; ! ?`, jamais d'espace ordinaire.
    #[test]
    fn le_francais_suit_la_typographie_francaise() {
        let fautes: Vec<_> = cles()
            .map(|c| (c, texte(Langue::Francais, c)))
            .filter(|(_, t)| {
                t.contains('\'')
                    || t.contains('"')
                    || t.char_indices().any(|(i, ch)| {
                        matches!(ch, ':' | ';' | '!' | '?')
                            && i > 0
                            && !t[..i].ends_with('\u{202f}')
                    })
            })
            .collect();
        assert!(fautes.is_empty(), "textes à corriger : {fautes:?}");
    }
}
