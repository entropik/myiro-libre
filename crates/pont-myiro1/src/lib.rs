//! Pont MYIRO-1 : logique de session au-dessus de `FDXSDK.dll`.
//!
//! La session impose l'ordre des paliers et le plafond fixé au lancement,
//! avant tout appel à la DLL (ADR 0005, docs/abi/).

use fdx_sys::{lire_infos_instrument, InfosInstrument, Port, Version, TAILLE_TAMPON_INFOS};
use pont_protocole::{ErreurPont, Palier};

/// Délai passé à `FDX_Connect`. Valeur utilisée par EIZO ; unité non établie
/// (fiche `docs/abi/FDX_Connect.md`).
pub const DELAI_CONNEXION: u32 = 10;

/// Les appels autorisés de `FDXSDK.dll`, un par export de la liste blanche.
/// Les erreurs sont les codes négatifs bruts de la DLL.
pub trait SdkMyiro1 {
    fn version(&mut self) -> Result<Version, i32>;
    /// Liste complète des instruments vus (appel en deux temps côté DLL).
    fn ports(&mut self) -> Result<Vec<Port>, i32>;
    fn connecter(&mut self, port: &Port, delai: u32) -> Result<(), i32>;
    fn infos(&mut self) -> Result<[u8; TAILLE_TAMPON_INFOS], i32>;
}

pub struct Session<S: SdkMyiro1> {
    sdk: S,
    plafond: Palier,
    atteint: Option<Palier>,
    ports: Vec<Port>,
}

impl<S: SdkMyiro1> Session<S> {
    pub fn new(sdk: S, plafond: Palier) -> Self {
        Session {
            sdk,
            plafond,
            atteint: None,
            ports: Vec::new(),
        }
    }

    pub fn sdk(&self) -> &S {
        &self.sdk
    }

    pub fn sdk_mut(&mut self) -> &mut S {
        &mut self.sdk
    }

    pub fn version(&mut self) -> Result<Version, ErreurPont> {
        self.autoriser(Palier::Version, None)?;
        let version = self.sdk.version().map_err(traduire)?;
        self.atteint = Some(Palier::Version);
        Ok(version)
    }

    /// Une liste vide veut dire « aucun instrument branché », pas une erreur.
    pub fn detecter(&mut self) -> Result<Vec<Port>, ErreurPont> {
        self.autoriser(Palier::Detection, Some(Palier::Version))?;
        self.ports = self.sdk.ports().map_err(traduire)?;
        self.atteint = Some(Palier::Detection);
        Ok(self.ports.clone())
    }

    /// Ouvre la session avec l'instrument n° `index` de la dernière détection
    /// et renvoie son identité.
    pub fn connecter(&mut self, index: usize) -> Result<InfosInstrument, ErreurPont> {
        self.autoriser(Palier::Connexion, Some(Palier::Detection))?;
        let port = *self.ports.get(index).ok_or(ErreurPont::InstrumentInconnu)?;
        self.sdk
            .connecter(&port, DELAI_CONNEXION)
            .map_err(traduire)?;
        self.atteint = Some(Palier::Connexion);
        let tampon = self.sdk.infos().map_err(traduire)?;
        Ok(lire_infos_instrument(&tampon))
    }

    fn autoriser(&self, demande: Palier, prealable: Option<Palier>) -> Result<(), ErreurPont> {
        if demande > self.plafond {
            return Err(ErreurPont::PalierNonAutorise {
                demande,
                plafond: self.plafond,
            });
        }
        if let Some(attendu) = prealable {
            if self.atteint < Some(attendu) {
                return Err(ErreurPont::EtatInvalide { attendu });
            }
        }
        Ok(())
    }
}

fn traduire(code: i32) -> ErreurPont {
    match code {
        -9992 => ErreurPont::ParametreRefuse,
        -9986 => ErreurPont::EtatIncompatible,
        code => ErreurPont::Sdk { code },
    }
}
