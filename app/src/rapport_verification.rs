//! Rapport de vérification d'un profil (ticket #41), en HTML autonome : un
//! seul fichier, styles intégrés, sans script ni lien externe, en noir et
//! gris pour rester lisible photocopié. Il est régénéré à chaque export
//! depuis les données rangées, jamais stocké, et tient dans une seule
//! langue. Tout texte variable est échappé.

mod donnees;
mod style;

use style::STYLE;

use bibliotheque::EtatVerification;
use pont_protocole::ConditionMesure;
use profilage::{Impression, Recette};
use verification::{Controle, Critere, Famille, Seuils, Statistiques};

use crate::textes::{texte, Langue};
use crate::verifier::{nom_version, CalculVerification};

pub use donnees::DonneesRapport;

/// `&`, `<`, `>`, `"` et `'` échappés.
pub fn echapper(t: &str) -> String {
    let mut s = String::with_capacity(t.len());
    for c in t.chars() {
        match c {
            '&' => s.push_str("&amp;"),
            '<' => s.push_str("&lt;"),
            '>' => s.push_str("&gt;"),
            '"' => s.push_str("&quot;"),
            '\'' => s.push_str("&#39;"),
            c => s.push(c),
        }
    }
    s
}

struct Page {
    html: String,
    langue: Langue,
}

impl Page {
    fn t(&self, cle: &str) -> String {
        texte(self.langue, cle).to_string()
    }

    /// Deux-points de la langue : espace fine insécable avant, en français.
    fn deux_points(&self) -> &'static str {
        match self.langue {
            Langue::Francais => "\u{202f}:",
            Langue::Anglais => ":",
        }
    }

    fn nombre(&self, v: f64, decimales: usize) -> String {
        let mut s = format!("{v:.decimales$}");
        if s.trim_start_matches('-')
            .chars()
            .all(|c| c == '0' || c == '.')
        {
            s = s.trim_start_matches('-').to_string();
        }
        if self.langue == Langue::Francais {
            s = s.replace('.', ",");
        }
        s
    }

    fn lab(&self, c: &colorimetrie::Lab) -> String {
        [c.l, c.a, c.b].map(|v| self.nombre(v, 1)).join(" ")
    }

    fn octets(&self, v: &[f64; 3], decimales: usize) -> String {
        v.map(|x| self.nombre(x * 255.0, decimales)).join(" ")
    }

    fn date(&self, horodatage: &str) -> String {
        let utc = format!(" {}", self.t("rapport.utc"));
        echapper(&horodatage.replacen('T', " ", 1).replace('Z', &utc))
    }

    fn ecrire(&mut self, morceau: &str) {
        self.html.push_str(morceau);
    }

    fn titre(&mut self, niveau: u8, cle: &str) {
        let t = echapper(&self.t(cle));
        self.ecrire(&format!("<h{niveau}>{t}</h{niveau}>\n"));
    }

    /// Une table de couples libellé, valeur (valeurs déjà échappées).
    fn couples(&mut self, lignes: &[(&str, String)]) {
        self.ecrire("<table>\n");
        for (cle, valeur) in lignes {
            let l = echapper(&self.t(cle));
            self.ecrire(&format!("<tr><th>{l}</th><td>{valeur}</td></tr>\n"));
        }
        self.ecrire("</table>\n");
    }

    fn inconnu(&self, raison: &str) -> String {
        echapper(&self.t(raison))
    }
}

/// Nom d'une recette tel que serde l'écrit (`rapide`, `standard`…).
fn nom_recette(r: Recette) -> String {
    serde_json::to_value(r)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default()
}

fn cle_critere(c: Critere) -> &'static str {
    match c {
        Critere::Moyenne => "verif.critere.moyenne",
        Critere::P95 => "verif.critere.p95",
        Critere::Max => "verif.critere.max",
    }
}

fn cle_famille(f: Famille) -> &'static str {
    match f {
        Famille::Fidelite => "verif.fidelite",
        Famille::Prediction => "verif.prediction",
    }
}

fn verdict(etat: EtatVerification) -> (&'static str, &'static str) {
    match etat {
        EtatVerification::Accepte => ("✓", "verif.etat.accepte"),
        EtatVerification::HorsTolerance => ("✗", "verif.etat.hors_tolerance"),
        EtatVerification::Incomplet => ("?", "verif.etat.incomplet"),
        EtatVerification::NonVerifie => ("?", "verif.etat.non_verifie"),
    }
}

fn seuils(p: &Page, s: &Seuils) -> [String; 3] {
    [s.moyenne, s.p95, s.max]
        .map(|v| v.map_or_else(|| p.inconnu("rapport.sans_seuil"), |v| p.nombre(v, 2)))
}

fn famille(p: &mut Page, cle: &str, stats: Option<&Statistiques>, raison: &str) {
    p.titre(3, cle);
    let Some(s) = stats else {
        let r = p.inconnu(raison);
        p.ecrire(&format!("<p>{r}</p>\n"));
        return;
    };
    let valeurs = [
        ("verif.critere.moyenne", p.nombre(s.moyenne, 2)),
        ("verif.critere.p95", p.nombre(s.p95, 2)),
        ("verif.critere.max", p.nombre(s.max, 2)),
        ("verif.plages", s.plages.to_string()),
    ];
    p.couples(&valeurs);
    let pires: Vec<String> = s
        .pires
        .iter()
        .map(|e| format!("{} ({})", echapper(&e.id), p.nombre(e.de00, 2)))
        .collect();
    let titre = echapper(&p.t("verif.pires"));
    let dp = p.deux_points();
    p.ecrire(&format!("<p>{titre}{dp} {}</p>\n", pires.join(" · ")));
}

fn controles(p: &mut Page, controles: &[Controle]) {
    p.titre(2, "rapport.controles");
    let entetes = [
        "rapport.famille",
        "rapport.critere",
        "rapport.limite",
        "rapport.valeur",
        "rapport.resultat",
    ]
    .map(|c| format!("<th>{}</th>", echapper(&p.t(c))))
    .concat();
    p.ecrire(&format!("<table>\n<tr>{entetes}</tr>\n"));
    for c in controles {
        let valeur = c
            .valeur
            .map_or_else(|| p.inconnu("verif.inconnu"), |v| p.nombre(v, 2));
        let resultat = match c.respecte {
            Some(true) => p.inconnu("verif.respecte"),
            Some(false) => p.inconnu("verif.depasse"),
            None => p.inconnu("verif.inconnu"),
        };
        let ligne = format!(
            "<tr><td>{}</td><td>{}</td><td class=\"n\">{}</td><td class=\"n\">{valeur}</td><td>{resultat}</td></tr>\n",
            echapper(&p.t(cle_famille(c.famille))),
            echapper(&p.t(cle_critere(c.critere))),
            p.nombre(c.limite, 2)
        );
        p.ecrire(&ligne);
    }
    p.ecrire("</table>\n");
}

fn plages(p: &mut Page, calcul: &CalculVerification) {
    p.titre(2, "rapport.toutes_plages");
    let calculee = echapper(&p.t("verif.calculee"));
    let entetes = [
        "rapport.plage",
        "rapport.rvb",
        "verif.demande",
        "verif.appareil",
        "verif.predite",
        "verif.mesuree",
        "rapport.ecart_fidelite",
        "rapport.ecart_prediction",
        "rapport.ecart_gamut",
    ]
    .map(|c| format!("<th>{}</th>", echapper(&p.t(c))))
    .concat();
    p.ecrire(&format!(
        "<p class=\"note\">{}{} {calculee}.</p>\n<table class=\"plages\">\n<tr>{entetes}</tr>\n",
        echapper(&p.t("verif.appareil")),
        p.deux_points()
    ));
    let inconnu = p.inconnu("verif.inconnu");
    for r in &calcul.resultats.plages {
        let ecart = |v: Option<f64>| v.map_or_else(|| inconnu.clone(), |v| p.nombre(v, 2));
        let ligne = format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td class=\"n\">{}</td><td class=\"n\">{}</td><td class=\"n\">{}</td></tr>\n",
            echapper(&r.id),
            p.octets(&r.rvb, 0),
            r.demande.as_ref().map_or_else(|| inconnu.clone(), |d| p.lab(d)),
            p.octets(&r.appareil, 1),
            p.lab(&r.prediction),
            p.lab(&r.mesure),
            ecart(r.ecart_fidelite),
            p.nombre(r.ecart_prediction, 2),
            ecart(r.ecart_gamut),
        );
        p.ecrire(&ligne);
    }
    p.ecrire("</table>\n");
}

/// Le rapport complet, dans la langue donnée.
pub fn rapport_html(d: &DonneesRapport, langue: Langue) -> String {
    let mut p = Page {
        html: String::new(),
        langue,
    };
    let jeu = nom_version(&d.verification.tolerances, langue);
    let titre = echapper(&p.t("rapport.titre"));
    p.ecrire(&format!(
        "<!DOCTYPE html>\n<html lang=\"{}\">\n<head>\n<meta charset=\"utf-8\">\n<title>{titre}</title>\n<style>{STYLE}</style>\n</head>\n<body>\n<h1>{titre}</h1>\n<p class=\"sous\">{} — {} — {} · {}</p>\n",
        langue.code(),
        echapper(&d.imprimante),
        echapper(&d.papier),
        echapper(&d.resolution),
        p.date(&d.verification.horodatage)
    ));
    // Verdict et règle d'acceptation.
    let (symbole, cle) = verdict(d.verification.etat);
    let mot = echapper(&p.t(cle).replace("{tolerances}", &jeu));
    p.ecrire(&format!("<p class=\"verdict\">{symbole} {mot}</p>\n"));
    let regle = echapper(&p.t("verif.regle"));
    p.ecrire(&format!("<p>{regle}</p>\n"));
    if let Some(date) = &d.recalcul_de {
        let r = echapper(&p.t("rapport.recalcul"));
        p.ecrire(&format!("<p class=\"note\">{r} {}</p>\n", p.date(date)));
    }
    if d.calcul.memes_couleurs_que_le_profil {
        let m = echapper(&p.t("verif.memes_couleurs"));
        p.ecrire(&format!("<p><strong>{m}</strong></p>\n"));
    }

    // Jeu de tolérances.
    p.titre(2, "verif.tolerances");
    let mut lignes = vec![
        ("verif.tolerances.nom", echapper(&jeu)),
        (
            "rapport.date",
            p.date(&d.verification.tolerances.horodatage),
        ),
        (
            "verif.tolerances.seuil_gamut",
            p.nombre(d.criteres.seuil_gamut, 2),
        ),
    ];
    for (cle, s) in [
        ("verif.fidelite", &d.criteres.fidelite),
        ("verif.prediction", &d.criteres.prediction),
    ] {
        let [m, c, x] = seuils(&p, s);
        let noms = [
            cle_critere(Critere::Moyenne),
            cle_critere(Critere::P95),
            cle_critere(Critere::Max),
        ]
        .map(|k| echapper(&p.t(k)));
        lignes.push((
            cle,
            format!("{} ≤ {m} · {} ≤ {c} · {} ≤ {x}", noms[0], noms[1], noms[2]),
        ));
    }
    p.couples(&lignes);

    // Deux familles de résultats, et les couleurs hors gamut à part.
    p.titre(2, "rapport.resultats");
    let sans_demande = d.calcul.evaluation.demande.is_none();
    let (raison_f, raison_h) = if sans_demande {
        ("verif.inconnu.sans_demande", "verif.inconnu.sans_demande")
    } else {
        (
            "verif.inconnu.aucune_plage_dans_le_gamut",
            "verif.inconnu.aucune_plage",
        )
    };
    let r = &d.calcul.resultats;
    famille(&mut p, "verif.fidelite", r.fidelite.as_ref(), raison_f);
    famille(&mut p, "verif.hors_gamut", r.hors_gamut.as_ref(), raison_h);
    famille(
        &mut p,
        "verif.prediction",
        r.prediction.as_ref(),
        "verif.inconnu",
    );
    controles(&mut p, &d.calcul.controles);

    // Profil, source, fiche, commandes.
    p.titre(2, "rapport.profil");
    let inconnu = p.inconnu("verif.inconnu");
    let recette = d.provenance.as_ref().map_or_else(
        || inconnu.clone(),
        |pr| echapper(&p.t(&format!("profil.recette.nom.{}", nom_recette(pr.recette)))),
    );
    let lignes = [
        ("rapport.nom", echapper(&d.profil.nom)),
        ("rapport.date", p.date(&d.profil.horodatage)),
        ("rapport.empreinte", echapper(&d.profil.empreinte)),
        ("rapport.recette", recette),
        ("rapport.argyll", echapper(&d.calcul.evaluation.version)),
    ];
    p.couples(&lignes);
    p.titre(2, "rapport.source");
    let supposee = |valeur: String, oui: bool| match oui {
        true => format!("{valeur} ({})", echapper(&p.t("rapport.supposee"))),
        false => valeur,
    };
    let c = match d.calcul.condition {
        ConditionMesure::M0 => "verif.condition.m0",
        ConditionMesure::M1 => "verif.condition.m1",
        ConditionMesure::M2 => "verif.condition.m2",
    };
    let condition = supposee(echapper(&p.t(c)), d.calcul.condition_supposee);
    let echelle = supposee(echapper(&p.t("rapport.rvb_echelle")), d.calcul.rvb_supposes);
    let mire = match &d.calcul.mire {
        Some(nom) => echapper(nom),
        None => echapper(&p.t("rapport.mire_du_profil")),
    };
    let lignes = [
        ("rapport.fichier", echapper(&d.calcul.fichier)),
        ("rapport.mire", mire),
        ("rapport.rvb", echelle),
        (
            "rapport.instrument",
            d.calcul
                .instrument
                .as_deref()
                .map_or_else(|| inconnu.clone(), echapper),
        ),
        ("verif.condition", condition),
    ];
    p.couples(&lignes);
    p.titre(2, "verif.fiche");
    let impression = match d.fiche.impression {
        Impression::ATraversLeProfil => "verif.fiche.impression.a_travers_le_profil",
        Impression::SansGestion => "verif.fiche.impression.sans_gestion",
    };
    let lignes = [
        ("verif.fiche.impression", echapper(&p.t(impression))),
        ("verif.fiche.logiciel", echapper(&d.fiche.logiciel)),
        (
            "verif.fiche.support_pilote",
            echapper(&d.fiche.support_pilote),
        ),
        ("verif.fiche.qualite", echapper(&d.fiche.qualite)),
        ("verif.fiche.encres", echapper(&d.fiche.encres)),
        ("verif.fiche.remarques", echapper(&d.fiche.remarques)),
    ];
    p.couples(&lignes);
    p.titre(2, "rapport.commandes");
    let commandes = echapper(&d.calcul.evaluation.commandes.join("\n"));
    p.ecrire(&format!("<pre>{commandes}</pre>\n"));
    plages(&mut p, &d.calcul);
    p.ecrire("</body>\n</html>\n");
    p.html
}
