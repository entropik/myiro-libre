//! Provenance d'une mesure dans l'en-tête CGATS : un mot-clé `MYIRO_LIBRE_*`
//! par champ, en texte lisible. Une donnée qualifiée s'écrit `inconnue`,
//! `confirmee:<valeur>` ou `supposee:<valeur>`.

use std::collections::HashMap;

use pont_protocole::{
    Calcul, ConditionMesure, ConditionsCalcul, Echantillonnage, Empreinte, Geometrie, Horodatage,
    Illuminant, Info, InstrumentMesurant, Observateur, Provenance,
};

use crate::{erreur, ErreurCgats};

pub(crate) const PREFIXE: &str = "MYIRO_LIBRE_";

fn qualifier<T>(info: &Info<T>, texte: impl Fn(&T) -> String) -> String {
    match info {
        Info::Confirmee(v) => format!("confirmee:{}", texte(v)),
        Info::Supposee(v) => format!("supposee:{}", texte(v)),
        Info::Inconnue => "inconnue".into(),
    }
}

fn dequalifier<T>(
    cle: &str,
    texte: &str,
    valeur: impl Fn(&str) -> Option<T>,
) -> Result<Info<T>, ErreurCgats> {
    let lue =
        |v: &str| valeur(v).ok_or_else(|| ErreurCgats(format!("{cle} : valeur « {v} » illisible")));
    if texte == "inconnue" {
        Ok(Info::Inconnue)
    } else if let Some(v) = texte.strip_prefix("confirmee:") {
        Ok(Info::Confirmee(lue(v)?))
    } else if let Some(v) = texte.strip_prefix("supposee:") {
        Ok(Info::Supposee(lue(v)?))
    } else {
        erreur(format!(
            "{cle} : « {texte} » n'est ni inconnue, ni confirmee:…, ni supposee:…"
        ))
    }
}

fn condition_texte(c: &ConditionMesure) -> String {
    format!("{c:?}")
}

pub(crate) fn condition_depuis(texte: &str) -> Option<ConditionMesure> {
    match texte {
        "M0" => Some(ConditionMesure::M0),
        "M1" => Some(ConditionMesure::M1),
        "M2" => Some(ConditionMesure::M2),
        _ => None,
    }
}

fn illuminant_texte(i: &Illuminant) -> String {
    match i {
        Illuminant::D50 => "D50".into(),
    }
}

fn observateur_texte(o: &Observateur) -> String {
    match o {
        Observateur::DeuxDegres => "2_degres".into(),
    }
}

fn conditions_calcul(nom: &str, info: &Info<ConditionsCalcul>, cles: &mut Vec<(String, String)>) {
    let statut = match info {
        Info::Confirmee(_) => "confirmee",
        Info::Supposee(_) => "supposee",
        Info::Inconnue => "inconnue",
    };
    cles.push((format!("CALCUL_{nom}"), statut.into()));
    if let Some(c) = info.valeur() {
        let conditions: Vec<String> = c
            .conditions_spectres
            .iter()
            .map(|i| qualifier(i, condition_texte))
            .collect();
        cles.push((format!("CALCUL_{nom}_CONDITIONS"), conditions.join(" ")));
        cles.push((
            format!("CALCUL_{nom}_LONGUEURS_ONDE"),
            qualifier(&c.longueurs_onde, |e| {
                format!("{}/{}", e.debut_nm, e.pas_nm)
            }),
        ));
        cles.push((
            format!("CALCUL_{nom}_ILLUMINANT"),
            qualifier(&c.illuminant_lab, illuminant_texte),
        ));
        cles.push((
            format!("CALCUL_{nom}_OBSERVATEUR"),
            qualifier(&c.observateur_lab, observateur_texte),
        ));
    }
}

/// Mots-clés de la provenance, sans le préfixe, dans l'ordre d'écriture.
pub(crate) fn ecrire(p: &Provenance) -> Vec<(String, String)> {
    let i = &p.instrument;
    let [a, b, c] = p.version_sdk;
    let mut cles: Vec<(String, String)> = vec![
        ("MODELE".into(), i.modele.clone()),
        ("NUMERO_SERIE".into(), i.numero_serie.to_string()),
        ("MICROLOGICIEL".into(), i.micrologiciel.clone()),
        ("CODE_PRODUIT".into(), i.code_produit.clone()),
        ("VERSION_SDK".into(), format!("{a}.{b}.{c}")),
        (
            "EMPREINTE_DLL".into(),
            qualifier(&p.empreinte_dll, |e| e.texte().to_string()),
        ),
        ("VERSION_PONT".into(), p.version_pont.clone()),
        ("ARCHITECTURE".into(), p.architecture.clone()),
        ("HORODATAGE".into(), p.horodatage.texte().to_string()),
        (
            "ETALONNAGE".into(),
            qualifier(&p.etalonnage, |h| h.texte().to_string()),
        ),
        (
            "GEOMETRIE".into(),
            match p.geometrie {
                Geometrie::Ponctuelle {} => "ponctuelle".into(),
                Geometrie::Bande { sens } => format!("bande {sens}"),
                Geometrie::Feuille {} => "feuille".into(),
            },
        ),
        ("CALCUL_LIBELLE".into(), p.calcul.libelle.clone()),
    ];
    conditions_calcul("DEMANDE", &p.calcul.demande, &mut cles);
    conditions_calcul("OBSERVE", &p.calcul.observe, &mut cles);
    cles
}

/// Relit la provenance écrite par [`ecrire`]. Il n'y manque rien : un champ
/// absent ou illisible fait refuser le fichier.
pub(crate) fn lire(cles: &HashMap<String, String>) -> Result<Provenance, ErreurCgats> {
    let champ = |nom: &str| -> Result<&str, ErreurCgats> {
        cles.get(&format!("{PREFIXE}{nom}"))
            .map(String::as_str)
            .ok_or_else(|| ErreurCgats(format!("provenance : {PREFIXE}{nom} manque")))
    };
    let illisible =
        |nom: &str, texte: &str| ErreurCgats(format!("{PREFIXE}{nom} : « {texte} » illisible"));

    let numero = champ("NUMERO_SERIE")?;
    let numero_serie = numero
        .parse()
        .map_err(|_| illisible("NUMERO_SERIE", numero))?;
    let sdk = champ("VERSION_SDK")?;
    let morceaux: Vec<u32> = sdk
        .split('.')
        .map(str::parse)
        .collect::<Result<_, _>>()
        .map_err(|_| illisible("VERSION_SDK", sdk))?;
    let version_sdk: [u32; 3] = morceaux
        .try_into()
        .map_err(|_| illisible("VERSION_SDK", sdk))?;
    let geometrie_texte = champ("GEOMETRIE")?;
    let geometrie = match geometrie_texte.split_once(' ') {
        None if geometrie_texte == "ponctuelle" => Geometrie::Ponctuelle {},
        None if geometrie_texte == "feuille" => Geometrie::Feuille {},
        Some(("bande", sens)) => Geometrie::Bande {
            sens: sens
                .parse()
                .map_err(|_| illisible("GEOMETRIE", geometrie_texte))?,
        },
        _ => return Err(illisible("GEOMETRIE", geometrie_texte)),
    };
    let horodatage = |t: &str| Horodatage::new(t).ok();

    Ok(Provenance {
        instrument: InstrumentMesurant {
            modele: champ("MODELE")?.into(),
            numero_serie,
            micrologiciel: champ("MICROLOGICIEL")?.into(),
            code_produit: champ("CODE_PRODUIT")?.into(),
        },
        version_sdk,
        empreinte_dll: dequalifier("EMPREINTE_DLL", champ("EMPREINTE_DLL")?, |t| {
            Empreinte::new(t).ok()
        })?,
        version_pont: champ("VERSION_PONT")?.into(),
        architecture: champ("ARCHITECTURE")?.into(),
        horodatage: Horodatage::new(champ("HORODATAGE")?)
            .map_err(|e| ErreurCgats(format!("{PREFIXE}HORODATAGE : {e}")))?,
        etalonnage: dequalifier("ETALONNAGE", champ("ETALONNAGE")?, horodatage)?,
        geometrie,
        calcul: Calcul {
            libelle: champ("CALCUL_LIBELLE")?.into(),
            demande: lire_conditions("DEMANDE", &champ)?,
            observe: lire_conditions("OBSERVE", &champ)?,
        },
    })
}

fn lire_conditions<'a>(
    nom: &str,
    champ: &impl Fn(&str) -> Result<&'a str, ErreurCgats>,
) -> Result<Info<ConditionsCalcul>, ErreurCgats> {
    let statut = champ(&format!("CALCUL_{nom}"))?;
    if statut == "inconnue" {
        return Ok(Info::Inconnue);
    }
    let cle = |suite: &str| format!("CALCUL_{nom}_{suite}");
    let conditions_texte = champ(&cle("CONDITIONS"))?;
    let conditions: Vec<Info<ConditionMesure>> = conditions_texte
        .split(' ')
        .map(|t| dequalifier(&cle("CONDITIONS"), t, condition_depuis))
        .collect::<Result<_, _>>()?;
    let conditions_spectres: [Info<ConditionMesure>; 3] = conditions.try_into().map_err(|_| {
        ErreurCgats(format!(
            "{PREFIXE}{} : trois conditions attendues",
            cle("CONDITIONS")
        ))
    })?;
    let conditions = ConditionsCalcul {
        conditions_spectres,
        longueurs_onde: dequalifier(
            &cle("LONGUEURS_ONDE"),
            champ(&cle("LONGUEURS_ONDE"))?,
            |t| {
                let (debut, pas) = t.split_once('/')?;
                Some(Echantillonnage {
                    debut_nm: debut.parse().ok()?,
                    pas_nm: pas.parse().ok()?,
                })
            },
        )?,
        illuminant_lab: dequalifier(&cle("ILLUMINANT"), champ(&cle("ILLUMINANT"))?, |t| {
            (t == "D50").then_some(Illuminant::D50)
        })?,
        observateur_lab: dequalifier(&cle("OBSERVATEUR"), champ(&cle("OBSERVATEUR"))?, |t| {
            (t == "2_degres").then_some(Observateur::DeuxDegres)
        })?,
    };
    match statut {
        "confirmee" => Ok(Info::Confirmee(conditions)),
        "supposee" => Ok(Info::Supposee(conditions)),
        autre => erreur(format!("{PREFIXE}CALCUL_{nom} : « {autre} » illisible")),
    }
}
