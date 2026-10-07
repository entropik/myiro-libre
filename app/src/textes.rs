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
    ("cartouche.inconnue", "Inconnue"),
    // Instrument : état dans la barre ({modele} et {etat} sont remplacés)
    ("instrument.recherche", "Recherche de l’instrument…"),
    ("instrument.barre", "{modele}, {etat}"),
    ("instrument.etat.connecte", "connecté"),
    ("instrument.etat.etalonnage_requis", "étalonnage requis"),
    ("instrument.etat.etalonne", "étalonné"),
    ("instrument.logiciel_absent", "Logiciel du fabricant absent"),
    (
        "raison.recherche",
        "Disponible quand la recherche de l’instrument est terminée.",
    ),
    // Instrument : écrans d’aide
    ("ecran.non_detecte.titre", "Aucun instrument détecté."),
    (
        "ecran.non_detecte.etape1",
        "Vérifiez que le câble USB est bien branché aux deux bouts.",
    ),
    (
        "ecran.non_detecte.etape2",
        "Branchez l’instrument directement sur l’ordinateur, sans concentrateur ni rallonge.",
    ),
    (
        "ecran.non_detecte.etape3",
        "Puis cliquez sur «\u{202f}Réessayer\u{202f}».",
    ),
    ("ecran.reessayer", "Réessayer"),
    (
        "ecran.choix.titre.logiciel_absent",
        "Le logiciel du fabricant du MYIRO-1 n’est pas installé sur ce poste.",
    ),
    (
        "ecran.choix.titre.logiciel_inutilisable",
        "Le logiciel du fabricant du MYIRO-1 est installé, mais il n’a pas pu être utilisé.",
    ),
    (
        "ecran.choix.explication",
        "myiro-libre s’appuie sur ce logiciel pour parler à l’instrument. S’il est installé ailleurs, choisissez son dossier.",
    ),
    ("ecran.choix.action", "Choisir le dossier…"),
    ("ecran.details", "Détails techniques"),
    // Problèmes : cause probable, puis une action
    (
        "probleme.logiciel_absent.cause",
        "Le logiciel du fabricant du MYIRO-1 n’a pas été trouvé.",
    ),
    (
        "probleme.logiciel_absent.action",
        "Choisissez le dossier où il est installé.",
    ),
    (
        "probleme.logiciel_inutilisable.cause",
        "Le logiciel du fabricant trouvé sur ce poste n’a pas pu être utilisé.",
    ),
    (
        "probleme.logiciel_inutilisable.action",
        "Choisissez le dossier d’un autre logiciel du fabricant.",
    ),
    (
        "probleme.pont_introuvable.cause",
        "Une partie de myiro-libre est introuvable.",
    ),
    (
        "probleme.pont_introuvable.action",
        "Réinstallez myiro-libre, puis réessayez.",
    ),
    (
        "probleme.pont_en_panne.cause",
        "Le dialogue avec l’instrument s’est interrompu.",
    ),
    (
        "probleme.pont_en_panne.action",
        "Réessayez. Si le problème revient, débranchez puis rebranchez l’instrument.",
    ),
    (
        "probleme.pont_bloque.cause",
        "L’instrument ne répondait plus et la liaison a été coupée. Il est peut-être resté dans un état incertain.",
    ),
    (
        "probleme.pont_bloque.action",
        "Débranchez l’instrument, attendez quelques secondes, rebranchez-le, puis réessayez.",
    ),
    (
        "probleme.detection_impossible.cause",
        "La recherche des instruments branchés a échoué.",
    ),
    (
        "probleme.detection_impossible.action",
        "Débranchez puis rebranchez l’instrument, et réessayez.",
    ),
    (
        "probleme.aucun_instrument.cause",
        "Aucun MYIRO-1 n’est détecté.",
    ),
    (
        "probleme.aucun_instrument.action",
        "Vérifiez le câble et branchez l’instrument directement sur l’ordinateur, puis réessayez.",
    ),
    (
        "probleme.connexion_impossible.cause",
        "Le MYIRO-1 est détecté, mais il ne répond pas à la connexion. Un autre logiciel l’utilise peut-être.",
    ),
    (
        "probleme.connexion_impossible.action",
        "Fermez les autres logiciels de mesure, débranchez puis rebranchez l’instrument, et réessayez.",
    ),
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
    ("cartouche.inconnue", "Unknown"),
    // Instrument: state in the top bar ({modele} and {etat} are replaced)
    ("instrument.recherche", "Looking for the instrument…"),
    ("instrument.barre", "{modele}, {etat}"),
    ("instrument.etat.connecte", "connected"),
    ("instrument.etat.etalonnage_requis", "calibration required"),
    ("instrument.etat.etalonne", "calibrated"),
    ("instrument.logiciel_absent", "Manufacturer software missing"),
    (
        "raison.recherche",
        "Available once the search for the instrument is over.",
    ),
    // Instrument: help screens
    ("ecran.non_detecte.titre", "No instrument detected."),
    (
        "ecran.non_detecte.etape1",
        "Check that the USB cable is firmly plugged in at both ends.",
    ),
    (
        "ecran.non_detecte.etape2",
        "Plug the instrument directly into the computer, without a hub or extension cable.",
    ),
    ("ecran.non_detecte.etape3", "Then click “Try again”."),
    ("ecran.reessayer", "Try again"),
    (
        "ecran.choix.titre.logiciel_absent",
        "The MYIRO-1 manufacturer’s software is not installed on this computer.",
    ),
    (
        "ecran.choix.titre.logiciel_inutilisable",
        "The MYIRO-1 manufacturer’s software is installed, but it could not be used.",
    ),
    (
        "ecran.choix.explication",
        "myiro-libre relies on this software to talk to the instrument. If it is installed elsewhere, choose its folder.",
    ),
    ("ecran.choix.action", "Choose the folder…"),
    ("ecran.details", "Technical details"),
    // Problems: likely cause, then one action
    (
        "probleme.logiciel_absent.cause",
        "The MYIRO-1 manufacturer’s software was not found.",
    ),
    (
        "probleme.logiciel_absent.action",
        "Choose the folder where it is installed.",
    ),
    (
        "probleme.logiciel_inutilisable.cause",
        "The manufacturer’s software found on this computer could not be used.",
    ),
    (
        "probleme.logiciel_inutilisable.action",
        "Choose the folder of another program from the manufacturer.",
    ),
    (
        "probleme.pont_introuvable.cause",
        "Part of myiro-libre cannot be found.",
    ),
    (
        "probleme.pont_introuvable.action",
        "Reinstall myiro-libre, then try again.",
    ),
    (
        "probleme.pont_en_panne.cause",
        "The connection with the instrument was interrupted.",
    ),
    (
        "probleme.pont_en_panne.action",
        "Try again. If it happens again, unplug and replug the instrument.",
    ),
    (
        "probleme.pont_bloque.cause",
        "The instrument stopped answering and the connection was cut. It may have been left in an uncertain state.",
    ),
    (
        "probleme.pont_bloque.action",
        "Unplug the instrument, wait a few seconds, plug it back in, then try again.",
    ),
    (
        "probleme.detection_impossible.cause",
        "The search for connected instruments failed.",
    ),
    (
        "probleme.detection_impossible.action",
        "Unplug and replug the instrument, then try again.",
    ),
    (
        "probleme.aucun_instrument.cause",
        "No MYIRO-1 is detected.",
    ),
    (
        "probleme.aucun_instrument.action",
        "Check the cable and plug the instrument directly into the computer, then try again.",
    ),
    (
        "probleme.connexion_impossible.cause",
        "The MYIRO-1 is detected but does not answer the connection. Another program may be using it.",
    ),
    (
        "probleme.connexion_impossible.action",
        "Close other measuring software, unplug and replug the instrument, then try again.",
    ),
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

    /// Chaque texte demandé par la page (`data-t`, `data-t-aria`) et chaque
    /// état de l'instrument affiché dans la barre existent au catalogue.
    #[test]
    fn la_page_ne_demande_que_des_cles_du_catalogue() {
        let page = include_str!("../interface/index.html");
        let mut demandees: Vec<String> = page
            .split("data-t")
            .skip(1)
            .filter_map(|reste| {
                let reste = reste.strip_prefix("-aria").unwrap_or(reste);
                let reste = reste.strip_prefix("=\"")?;
                reste.split('"').next().map(String::from)
            })
            .collect();
        for etat in ["connecte", "etalonnage_requis", "etalonne"] {
            demandees.push(format!("instrument.etat.{etat}"));
        }
        demandees.extend(
            [
                "instrument.recherche",
                "instrument.aucun",
                "instrument.barre",
                "instrument.logiciel_absent",
                "ecran.choix.titre.logiciel_absent",
                "ecran.choix.titre.logiciel_inutilisable",
            ]
            .map(String::from),
        );
        assert!(
            demandees.len() > 40,
            "clés lues dans la page : {demandees:?}"
        );
        let inconnues: Vec<_> = demandees
            .iter()
            .filter(|c| !cles().any(|k| k == c.as_str()))
            .collect();
        assert!(
            inconnues.is_empty(),
            "clés absentes du catalogue : {inconnues:?}"
        );
    }

    /// Un texte qui cite un bouton le met entre guillemets français :
    /// « Puis cliquez sur «\u{202f}Réessayer\u{202f}». »
    #[test]
    fn un_bouton_cite_est_entre_guillemets() {
        for bouton in ["ecran.reessayer", "ecran.choix.action"] {
            let libelle = texte(Langue::Francais, bouton);
            let cite = format!("«\u{202f}{libelle}\u{202f}»");
            let fautes: Vec<_> = cles()
                .filter(|c| *c != bouton)
                .map(|c| texte(Langue::Francais, c))
                .filter(|t| t.contains(libelle) && !t.contains(&cite))
                .collect();
            assert!(
                fautes.is_empty(),
                "« {libelle} » sans guillemets : {fautes:?}"
            );
        }
    }

    #[test]
    fn chaque_cle_existe_en_francais_et_en_anglais() {
        assert_eq!(cles_manquantes(), vec![]);
    }

    /// Typographie française : apostrophe courbe, guillemets français « » (ni
    /// droits ni anglais) avec espace fine insécable (U+202F) à l'intérieur,
    /// espace fine insécable avant `: ; ! ?`, jamais d'espace ordinaire.
    #[test]
    fn le_francais_suit_la_typographie_francaise() {
        let fautes: Vec<_> = cles()
            .map(|c| (c, texte(Langue::Francais, c)))
            .filter(|(_, t)| {
                t.contains(['\'', '"', '“', '”'])
                    || t.char_indices().any(|(i, ch)| {
                        (matches!(ch, ':' | ';' | '!' | '?' | '»')
                            && i > 0
                            && !t[..i].ends_with('\u{202f}'))
                            || (ch == '«' && !t[i + ch.len_utf8()..].starts_with('\u{202f}'))
                    })
            })
            .collect();
        assert!(fautes.is_empty(), "textes à corriger : {fautes:?}");
    }
}
