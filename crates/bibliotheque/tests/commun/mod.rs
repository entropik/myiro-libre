//! Mesures fictives partagées par les tests de la bibliothèque (n° 12345678).

#![allow(dead_code)]

use pont_protocole::{
    Calcul, ConditionMesure, ConditionsCalcul, DonneesBrutes, Echantillonnage, Empreinte,
    Geometrie, Horodatage, Illuminant, Info, InstrumentMesurant, Lab, Mesure, Observateur, Plage,
    Provenance, Spectre,
};

/// Plage fictive : chaque valeur est distincte, pour voir la moindre perte.
pub fn plage(graine: f32) -> Plage {
    let spectre = |decalage: f32| {
        Spectre::new(
            (0..36)
                .map(|i| graine + decalage + i as f32 * 0.013_7)
                .collect(),
        )
        .unwrap()
    };
    Plage::new(
        [spectre(0.0), spectre(0.001), spectre(-0.002)],
        DonneesBrutes::new((0..152).map(|i| graine * 1000.0 + i as f32 * 3.5).collect()).unwrap(),
        [
            Lab::new([52.31, 1.07, -2.389]).unwrap(),
            Lab::new([52.4, 1.9, -6.01]).unwrap(),
            Lab::new([52.0, 1.1, -1.8]).unwrap(),
        ],
    )
    .unwrap()
}

pub fn provenance(numero_serie: u32, horodatage: &str, geometrie: Geometrie) -> Provenance {
    Provenance {
        instrument: InstrumentMesurant {
            modele: "MYIRO-1".into(),
            numero_serie,
            micrologiciel: "1.02.0005".into(),
            code_produit: "9C1D".into(),
        },
        version_sdk: [1, 0, 1],
        empreinte_dll: Info::Confirmee(Empreinte::new("ab".repeat(32)).unwrap()),
        version_pont: "0.2.0".into(),
        architecture: "x86".into(),
        horodatage: Horodatage::new(horodatage).unwrap(),
        // Étalonnage et conditions observées : inconnus, ils doivent le rester.
        etalonnage: Info::Inconnue,
        geometrie,
        calcul: Calcul {
            libelle: "spectres : M0/M1/M2 ; Lab : D50, 2°".into(),
            demande: Info::Confirmee(ConditionsCalcul {
                conditions_spectres: [
                    Info::Confirmee(ConditionMesure::M0),
                    Info::Supposee(ConditionMesure::M1),
                    Info::Inconnue,
                ],
                longueurs_onde: Info::Confirmee(Echantillonnage {
                    debut_nm: 380,
                    pas_nm: 10,
                }),
                illuminant_lab: Info::Confirmee(Illuminant::D50),
                observateur_lab: Info::Supposee(Observateur::DeuxDegres),
            }),
            observe: Info::Inconnue,
        },
    }
}

pub fn ponctuelle(numero_serie: u32, horodatage: &str) -> Mesure {
    Mesure::new(
        vec![plage(0.21)],
        provenance(numero_serie, horodatage, Geometrie::Ponctuelle {}),
    )
    .unwrap()
}

pub fn bande(horodatage: &str) -> Mesure {
    Mesure::new(
        vec![plage(0.1), plage(0.4), plage(0.85)],
        provenance(12345678, horodatage, Geometrie::Bande { sens: 2 }),
    )
    .unwrap()
}
