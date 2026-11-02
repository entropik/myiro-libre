//! Le moment du rapport, à l'heure locale du poste : son année donne le
//! numéro, son texte va dans le cartouche (« 30 octobre 2026, 14 h 05 »,
//! « 30 October 2026, 14:05 »).

use std::time::{SystemTime, UNIX_EPOCH};

use super::t;
use crate::textes::Langue;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Jour {
    pub annee: i32,
    /// 1 à 12.
    pub mois: u8,
    pub jour: u8,
    /// 0 à 23, heure locale.
    pub heure: u8,
    pub minute: u8,
}

impl Jour {
    /// La date et l'heure civiles d'un instant, décalé de `decalage_s`
    /// secondes par rapport au temps universel (fuseau et heure d'été du
    /// poste).
    pub fn de(instant: SystemTime, decalage_s: i64) -> Jour {
        let secondes = instant
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs() as i64)
            + decalage_s;
        let jours = secondes.div_euclid(86_400);
        let dans_le_jour = secondes.rem_euclid(86_400);
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
        Jour {
            annee,
            mois,
            jour,
            heure: (dans_le_jour / 3_600) as u8,
            minute: (dans_le_jour % 3_600 / 60) as u8,
        }
    }

    /// Maintenant, à l'heure locale du poste (fuseau et heure d'été lus
    /// dans Windows).
    pub fn maintenant() -> Jour {
        let decalage = chrono::Local::now().offset().local_minus_utc();
        Jour::de(SystemTime::now(), i64::from(decalage))
    }

    pub fn texte(self, langue: Langue) -> String {
        t(langue, "rapport.date_longue")
            .replace("{j}", &self.jour.to_string())
            .replace(
                "{mois}",
                &t(langue, &format!("rapport.mois.{}", self.mois.clamp(1, 12))),
            )
            .replace("{a}", &self.annee.to_string())
            .replace("{h}", &self.heure.to_string())
            .replace("{mn}", &format!("{:02}", self.minute))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const fn jour(annee: i32, mois: u8, jour: u8, heure: u8, minute: u8) -> Jour {
        Jour {
            annee,
            mois,
            jour,
            heure,
            minute,
        }
    }

    #[test]
    fn un_instant_donne_son_jour_et_son_heure() {
        let utc = |s| Jour::de(UNIX_EPOCH + Duration::from_secs(s), 0);
        assert_eq!(utc(0), jour(1970, 1, 1, 0, 0));
        // 30 octobre 2026, 23 h 59 UTC.
        assert_eq!(utc(1_793_404_740), jour(2026, 10, 30, 23, 59));
        // 29 février 2028.
        assert_eq!(utc(1_835_395_200), jour(2028, 2, 29, 0, 0));
    }

    /// Le décalage du poste compte : 23 h 59 UTC le 30 octobre est déjà le
    /// 31 à Paris (UTC+1), et le 31 décembre 23 h 30 à Paris est encore en
    /// 2026 quand l'heure universelle est en 2027… ou l'inverse.
    #[test]
    fn l_heure_locale_change_le_jour_et_l_annee() {
        let instant = UNIX_EPOCH + Duration::from_secs(1_793_404_740);
        assert_eq!(Jour::de(instant, 3_600), jour(2026, 10, 31, 0, 59));
        assert_eq!(Jour::de(instant, -5 * 3_600), jour(2026, 10, 30, 18, 59));
        // 1er janvier 2027, 0 h 30 UTC : 31 décembre 2026, 19 h 30 à New York.
        let nouvel_an = UNIX_EPOCH + Duration::from_secs(1_798_763_400);
        assert_eq!(Jour::de(nouvel_an, 0), jour(2027, 1, 1, 0, 30));
        assert_eq!(Jour::de(nouvel_an, -5 * 3_600), jour(2026, 12, 31, 19, 30));
    }

    #[test]
    fn le_texte_porte_la_date_et_l_heure() {
        let j = jour(2026, 11, 2, 9, 5);
        assert_eq!(
            j.texte(Langue::Francais),
            "2 novembre 2026, 9\u{a0}h\u{a0}05"
        );
        assert_eq!(j.texte(Langue::Anglais), "2 November 2026, 9:05");
    }
}
