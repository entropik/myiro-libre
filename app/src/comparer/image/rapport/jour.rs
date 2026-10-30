//! Le jour du rapport : son année donne le numéro, son texte va dans le
//! cartouche (« 30 octobre 2026 », « 30 October 2026 »).

use std::time::{SystemTime, UNIX_EPOCH};

use super::t;
use crate::textes::Langue;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Jour {
    pub annee: i32,
    /// 1 à 12.
    pub mois: u8,
    pub jour: u8,
}

impl Jour {
    /// Le jour civil (temps universel) d'un instant.
    pub fn de(instant: SystemTime) -> Jour {
        let jours = instant
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs() / 86_400) as i64;
        // Jours depuis le 1er janvier 1970 vers une date du calendrier
        // grégorien (algorithme de Howard Hinnant).
        let z = jours + 719_468;
        let ere = z.div_euclid(146_097);
        let jde = z.rem_euclid(146_097);
        let ade = (jde - jde / 1_460 + jde / 36_524 - jde / 146_096) / 365;
        let jda = jde - (365 * ade + ade / 4 - ade / 100);
        let mp = (5 * jda + 2) / 153;
        let jour = (jda - (153 * mp + 2) / 5 + 1) as u8;
        let mois = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
        let annee = (ade + ere * 400 + i64::from(mois <= 2)) as i32;
        Jour { annee, mois, jour }
    }

    pub fn aujourdhui() -> Jour {
        Jour::de(SystemTime::now())
    }

    pub fn texte(self, langue: Langue) -> String {
        t(langue, "rapport.date_longue")
            .replace("{j}", &self.jour.to_string())
            .replace(
                "{mois}",
                &t(langue, &format!("rapport.mois.{}", self.mois.clamp(1, 12))),
            )
            .replace("{a}", &self.annee.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn un_instant_donne_son_jour() {
        let jour = |s| Jour::de(UNIX_EPOCH + Duration::from_secs(s));
        assert_eq!(
            jour(0),
            Jour {
                annee: 1970,
                mois: 1,
                jour: 1
            }
        );
        // 30 octobre 2026, 23 h 59 UTC.
        assert_eq!(
            jour(1_793_404_740),
            Jour {
                annee: 2026,
                mois: 10,
                jour: 30
            }
        );
        // 29 février 2028.
        assert_eq!(
            jour(1_835_395_200),
            Jour {
                annee: 2028,
                mois: 2,
                jour: 29
            }
        );
    }
}
