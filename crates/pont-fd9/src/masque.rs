//! Masque de ce qui identifie l'instrument (adresse, identifiant, n° de
//! série, adresse MAC) dans tout ce que le pont écrit sur sa sortie
//! d'erreur : journal recopié et trace (`--trace`). L'application range
//! cette sortie dans un fichier local (`mesures-fd9/traces/`) : il ne doit
//! jamais contenir d'identifiant réel, même si une ligne en cite un.
//!
//! Le pont retient chaque identifiant dès qu'il le connaît (détection,
//! connexion, identité) ; chaque ligne écrite ensuite le voit remplacé par
//! son nom (« <adresse> », « <n° de série> »…).

use std::sync::{Arc, Mutex};

/// Plus court identifiant masqué : en deçà, il se confondrait avec des
/// nombres ordinaires d'une ligne.
const LONGUEUR_MIN: usize = 4;

/// Identifiants retenus, partagés entre la session et les sorties de
/// l'exécutable.
#[derive(Clone, Debug, Default)]
pub struct Masque(Arc<Mutex<Vec<(String, &'static str)>>>);

impl Masque {
    fn liste(&self) -> std::sync::MutexGuard<'_, Vec<(String, &'static str)>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Retient `secret`, qui sera écrit `<nom>` dans toute ligne masquée.
    /// Les blancs et les octets nuls autour sont retirés ; un texte trop
    /// court est ignoré.
    pub fn retenir(&self, secret: &str, nom: &'static str) {
        let secret = secret.trim_matches(|c: char| c == '\0' || c.is_whitespace());
        if secret.chars().count() < LONGUEUR_MIN {
            return;
        }
        let mut liste = self.liste();
        if liste.iter().any(|(s, _)| s == secret) {
            return;
        }
        liste.push((secret.to_string(), nom));
        // Le plus long d'abord : un identifiant contenu dans un autre ne
        // laisse rien du plus long.
        liste.sort_by_key(|(s, _)| std::cmp::Reverse(s.len()));
    }

    /// `ligne`, chaque identifiant retenu remplacé par son nom.
    pub fn appliquer(&self, ligne: &str) -> String {
        let mut masquee = ligne.to_string();
        for (secret, nom) in self.liste().iter() {
            if masquee.contains(secret.as_str()) {
                masquee = masquee.replace(secret.as_str(), &format!("<{nom}>"));
            }
        }
        masquee
    }
}
