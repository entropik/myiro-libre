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
    ("tache.imprimer", "Imprimer"),
    ("marque.sous_titre", "poste couleur et impression"),
    ("instrument.libelle", "Instrument"),
    ("instrument.aucun", "Aucun instrument"),
    ("theme.libelle", "Thème"),
    ("theme.clair", "Clair"),
    ("theme.sombre", "Sombre"),
    ("langue.libelle", "Langue"),
    // Bibliothèque, à gauche
    ("bibliotheque.recherche", "Rechercher"),
    ("bibliotheque.recherche_aide", "Imprimante, papier ou résolution"),
    (
        "bibliotheque.aucun_resultat",
        "Rien ne correspond à cette recherche.",
    ),
    (
        "bibliotheque.demonstration",
        "Démonstration, données fictives",
    ),
    ("bibliotheque.mesures", "Mesures"),
    ("bibliotheque.erreur.nom_vide", "Donnez-lui un nom."),
    (
        "bibliotheque.erreur.nom_pris",
        "Une autre condition d’impression porte déjà ce nom.",
    ),
    (
        "bibliotheque.erreur.autre",
        "La bibliothèque n’a pas pu faire cette opération.",
    ),
    (
        "bibliotheque.erreur.ouverture",
        "La bibliothèque n’a pas pu s’ouvrir.",
    ),
    (
        "bibliotheque.erreur.sauvegarde_invalide",
        "Ce fichier n’est pas une sauvegarde utilisable. La bibliothèque n’a pas changé.",
    ),
    (
        "bibliotheque.erreur.cgats_illisible",
        "Ce fichier n’a pas pu être lu comme un CGATS, ou il ne dit pas sa condition de mesure. Rien n’a été deviné.",
    ),
    (
        "bibliotheque.erreur.cgats_trop_gros",
        "Ce fichier est trop gros pour un fichier de mesures (plus de 16\u{202f}Mio). Rien n’a été importé.",
    ),
    ("bibliotheque.erreur.export", "Le fichier n’a pas pu être écrit."),
    // Parc : imprimantes, papiers, résolutions et profils
    (
        "parc.vide",
        "Aucune imprimante. Ajoutez la première imprimante de l’atelier, puis ses papiers.",
    ),
    ("parc.ajouter_imprimante", "Ajouter une imprimante"),
    ("parc.titre", "Imprimantes et papiers"),
    ("parc.non_classees", "Non classées"),
    ("parc.ajouter_papier", "Ajouter un papier"),
    ("parc.ajouter_resolution", "Ajouter une résolution"),
    ("parc.ajout.imprimante", "Nouvelle imprimante"),
    ("parc.ajout.papier", "Nouveau papier pour {}"),
    ("parc.ajout.resolution", "Nouvelle résolution pour {}"),
    ("parc.champ.imprimante", "Nom de l’imprimante"),
    ("parc.champ.papier", "Nom du papier"),
    ("parc.champ.resolution", "Résolution d’impression"),
    ("parc.ajouter", "Ajouter"),
    ("parc.annuler", "Annuler"),
    ("parc.compte.papier", "papier"),
    ("parc.compte.papiers", "papiers"),
    ("parc.age.aucun", "aucun"),
    ("parc.age.ancien", "{} · ancien"),
    ("parc.age.plus_d_un_an", "plus d’un an"),
    ("parc.verdict.contexte", "Profil actuel"),
    ("parc.verdict.verifie", "Profil vérifié"),
    ("parc.verdict.non_verifie", "Profil à vérifier"),
    ("parc.verdict.calcule_a_verifier", "Profil calculé, à vérifier"),
    ("parc.verdict.importe_a_verifier", "Profil importé, à vérifier"),
    ("parc.verdict.hors", "Profil hors tolérance"),
    ("parc.verdict.incomplet", "Vérification incomplète"),
    ("parc.verdict.aucun", "Aucun profil"),
    (
        "parc.verdict.aucun_contexte",
        "Rangez un profil existant pour cette résolution",
    ),
    ("parc.espace.rvb", "RVB"),
    ("parc.espace.cmjn", "CMJN"),
    ("parc.installer", "Installer dans Windows"),
    (
        "parc.installer.raison",
        "Aucun profil dans cette résolution. Rangez d’abord un profil existant.",
    ),
    ("parc.exporter", "Exporter"),
    ("parc.ranger", "Ranger un profil existant"),
    ("parc.range", "Profil rangé. Il devient le profil actuel."),
    (
        "parc.en_cours",
        "Action en cours sur ce profil. Les boutons reviennent dès qu’elle est finie.",
    ),
    (
        "parc.deja_installe",
        "Ce profil est déjà installé dans Windows, à l’identique.",
    ),
    ("parc.filtre.icc", "Profil ICC"),
    ("parc.historique", "Historique"),
    ("parc.historique.actuel", "actuel"),
    ("parc.historique.remplace", "remplacé"),
    ("parc.historique.vide", "Aucun profil pour l’instant."),
    ("parc.etat.non_verifie", "à vérifier"),
    ("parc.etat.accepte", "vérifié"),
    ("parc.etat.hors_tolerance", "hors tolérance"),
    ("parc.etat.incomplet", "incomplet"),
    ("parc.mesures.vide", "Aucune mesure dans cette résolution."),
    ("parc.provenance", "Provenance"),
    ("parc.origine", "Origine"),
    ("parc.origine.profil_existant", "profil existant"),
    ("parc.origine.calcul", "calcul de myiro-libre"),
    ("parc.nom", "Fichier"),
    ("parc.empreinte", "Empreinte"),
    ("parc.instrument", "Instrument"),
    ("parc.mire", "Mire"),
    ("parc.inconnu", "inconnu"),
    ("parc.inconnue", "inconnue"),
    ("parc.fiche", "Fiche d’impression"),
    ("parc.fiche.reglages", "Réglages du pilote"),
    ("parc.windows", "Windows"),
    ("parc.windows.etat", "Installation"),
    ("parc.windows.non_installe", "pas installé"),
    ("parc.windows.sans_profil", "aucun profil"),
    (
        "parc.erreur.imprimante_inconnue",
        "Cette imprimante n’existe plus dans la bibliothèque.",
    ),
    (
        "parc.erreur.papier_inconnu",
        "Ce papier n’existe plus dans la bibliothèque.",
    ),
    (
        "parc.erreur.reglage_inconnu",
        "Cette résolution n’existe plus dans la bibliothèque.",
    ),
    (
        "parc.erreur.profil_inconnu",
        "Ce profil n’existe plus dans la bibliothèque.",
    ),
    (
        "parc.erreur.profil_illisible",
        "Ce fichier n’est pas un profil ICC RVB ou CMJN. Choisissez un fichier .icc ou .icm d’imprimante.",
    ),
    (
        "parc.erreur.fichier_illisible",
        "Le fichier n’a pas pu être lu. Vérifiez qu’il existe encore et qu’il n’est pas ouvert ailleurs.",
    ),
    // Calcul d’un profil (ticket #40)
    (
        "profil.titre",
        "Calculer un profil",
    ),
    (
        "profil.source",
        "Mesure",
    ),
    (
        "profil.condition",
        "Condition de mesure",
    ),
    (
        "profil.recette",
        "Recette de calcul",
    ),
    (
        "profil.recette.rapide",
        "Rapide\u{202f}: pour un essai, le plus court (environ 1\u{a0}min pour 2\u{202f}520 plages)",
    ),
    (
        "profil.recette.standard",
        "Standard\u{202f}: conseillée pour la production (environ 3\u{a0}min pour 2\u{202f}520 plages)",
    ),
    (
        "profil.recette.approfondi",
        "Approfondi\u{202f}: en cours de réglage, pas encore disponible",
    ),
    (
        "profil.avance",
        "Avancé\u{202f}: commande équivalente",
    ),
    (
        "profil.calculer",
        "Calculer le profil",
    ),
    (
        "profil.en_cours",
        "Calcul du profil en cours. Cela peut prendre plusieurs minutes.",
    ),
    (
        "profil.etape.preparation",
        "Préparation des mesures",
    ),
    (
        "profil.etape.profil",
        "Calcul du profil (colprof)\u{202f}: courbes",
    ),
    (
        "profil.etape.table",
        "Calcul du profil (colprof)\u{202f}: table principale",
    ),
    (
        "profil.etape.gamut",
        "Calcul du profil (colprof)\u{202f}: correspondance des gamuts",
    ),
    (
        "profil.etape.inverses",
        "Calcul du profil (colprof)\u{202f}: tables inverses",
    ),
    (
        "profil.etape.limite",
        "Calcul du profil (colprof)\u{202f}: limite du gamut",
    ),
    (
        "profil.progression",
        "Avancement du calcul",
    ),
    (
        "profil.progression.etape",
        "Calcul du profil, étape {n} sur {total}",
    ),
    (
        "profil.estimation.restante",
        "environ {duree} restantes (estimé)",
    ),
    (
        "profil.estimation.depassee",
        "plus long que le dernier calcul de cette recette (estimé)",
    ),
    (
        "profil.etape.controle",
        "Contrôle de précision (profcheck)",
    ),
    (
        "profil.ecoule",
        "temps écoulé\u{202f}: {duree}",
    ),
    (
        "profil.argyll.ligne",
        "Dernière ligne d’ArgyllCMS\u{202f}: {ligne}",
    ),
    (
        "profil.argyll.pourcentage",
        "Dernière ligne d’ArgyllCMS\u{202f}: {ligne} — {pourcentage}\u{202f}% de cette partie",
    ),
    (
        "profil.fait",
        "Profil calculé. Écart moyen {moyenne} ΔE00, maximum {max}. Il reste à vérifier.",
    ),
    (
        "profil.supposee",
        "supposée",
    ),
    (
        "profil.refus.condition_absente",
        "Cette mesure ne contient pas cette condition de mesure. Choisissez une condition proposée dans la liste.",
    ),
    (
        "profil.refus.rvb_inconnus",
        "Cette mesure ne dit pas quelles valeurs RVB ont été imprimées. Importez un export CGATS de FD-S2w ou de MYIROtools qui contient les colonnes RVB.",
    ),
    (
        "profil.refus.lab_inconnu",
        "Une plage de cette mesure n’a pas de valeur Lab pour cette condition. Choisissez une autre condition, ou mesurez à nouveau la mire.",
    ),
    (
        "profil.refus.rvb_hors_echelle",
        "Une plage de cette mesure a une valeur RVB hors de l’échelle prévue. Vérifiez le fichier exporté, puis importez-le à nouveau.",
    ),
    (
        "profil.refus.vide",
        "Cette mesure ne contient aucune plage. Importez un autre fichier.",
    ),
    (
        "profil.refus.aucune_condition",
        "Aucune condition de mesure de ce fichier n’a toutes ses plages en Lab. Importez un export complet de FD-S2w ou de MYIROtools.",
    ),
    (
        "profil.erreur.argyll_introuvable",
        "ArgyllCMS 3.5.0 est introuvable. Placez-le dans le dossier argyll à côté de l’application, puis relancez l’application.",
    ),
    (
        "profil.erreur.argyll_version",
        "ArgyllCMS {version} a été trouvé, mais seule la version 3.5.0 est validée. Placez ArgyllCMS 3.5.0 dans le dossier argyll à côté de l’application, puis relancez l’application.",
    ),
    (
        "profil.erreur.argyll_illisible",
        "ArgyllCMS ne répond pas comme prévu\u{202f}: sa version n’a pas pu être lue. Remplacez le dossier argyll à côté de l’application par une copie complète d’ArgyllCMS 3.5.0, puis relancez l’application.",
    ),
    (
        "profil.erreur.calcul",
        "Le calcul du profil a échoué dans ArgyllCMS. Le profil actuel n’a pas changé. Vérifiez la mesure, puis réessayez.",
    ),
    (
        "profil.erreur.recette_en_reglage",
        "La recette Approfondi est en cours de réglage. Choisissez Rapide ou Standard.",
    ),
    (
        "profil.erreur.interrompu",
        "Le calcul a été interrompu avant la fin (programme ArgyllCMS arrêté, ou poste mis en veille). Le profil actuel n’a pas changé. Relancez le calcul.",
    ),
    (
        "profil.erreur.controle",
        "Le contrôle du profil calculé a échoué dans ArgyllCMS. Rien n’a été rangé, le profil actuel n’a pas changé. Réessayez.",
    ),
    (
        "profil.erreur.fichier",
        "Un fichier de travail du calcul n’a pas pu être lu ou écrit. Rien n’a été rangé. Réessayez, puis redémarrez l’ordinateur si l’erreur revient.",
    ),
    (
        "profil.erreur.mesure_ailleurs",
        "Cette mesure est rangée dans une autre résolution. Choisissez une mesure de cette résolution.",
    ),
    ("profil.erreur.cause", "Message d’ArgyllCMS\u{202f}: {cause}"),
    ("profil.recette.nom.rapide", "Rapide"),
    ("profil.recette.nom.standard", "Standard"),
    ("profil.recette.nom.approfondi", "Approfondi"),
    (
        "profil.calculer.raison",
        "Aucune mesure de cette résolution ne peut servir à calculer un profil. La raison est dite sous chaque mesure.",
    ),
    (
        "profil.sans_mesure",
        "Importez d’abord un export CGATS de FD-S2w ou de MYIROtools dans cette résolution.",
    ),
    ("profil.version", "ArgyllCMS"),
    ("profil.journal", "Journal d’ArgyllCMS"),
    ("profil.rvb", "Valeurs RVB"),
    ("profil.rvb.supposee", "échelle 0 à 255 supposée"),
    ("profil.avance.provenance", "Avancé\u{202f}: commande et journal"),
    (
        "parc.erreur.inattendue",
        "L’action s’est arrêtée sur une erreur inattendue. Choisissez à nouveau cette résolution dans la bibliothèque, puis réessayez.",
    ),
    ("profil.precision", "Précision du calcul"),
    ("profil.duree", "Durée du calcul"),
    (
        "profil.precision.valeur",
        "{moyenne} en moyenne, {max} au plus (ΔE00)",
    ),
    // Contrôle de cohérence des mesures (ticket #1)
    (
        "profil.refus.incoherent",
        "Les mesures ne correspondent pas aux plages de la mire\u{202f}: le profil serait faux. Plages en cause\u{202f}: {plages}. Relisez la mire, vérifiez le sens de la feuille et l’ordre de lecture, et que ce fichier est bien celui de cette mire.",
    ),
    ("profil.coherence.autres", "et {n} autres"),
    ("profil.coherence.autre", "et 1 autre"),
    (
        "profil.coherence.decalage",
        "Les mesures semblent décalées de {n} plages.",
    ),
    (
        "profil.coherence.decalage_une",
        "Les mesures semblent décalées d’une plage.",
    ),
    (
        "profil.coherence.criteres",
        "Contrôles qui les signalent\u{202f}: {criteres}.",
    ),
    (
        "profil.coherence.critere.degrade_gris",
        "dégradé gris, du foncé au clair",
    ),
    ("profil.coherence.critere.blanc_noir", "blanc papier et noir"),
    ("profil.coherence.critere.doublons", "plages répétées"),
    ("profil.coherence.critere.voisines", "plages voisines"),
    ("profil.coherence", "Mesures contrôlées"),
    ("profil.coherence.accepte", "cohérentes"),
    ("profil.coherence.non_fait", "Contrôles non faits"),
    (
        "profil.coherence.non_controle",
        "non, profil calculé avant ce contrôle",
    ),
    // Installation dans Windows
    ("installation.installe", "Installé dans Windows"),
    (
        "installation.nom_invalide",
        "Le nom de ce profil ne convient pas à Windows. Il doit être un simple nom de fichier finissant par .icc ou .icm.",
    ),
    (
        "installation.deja_present",
        "Un autre profil du même nom est déjà installé dans Windows\u{202f}: renommez ce profil ou retirez l’autre.",
    ),
    (
        "installation.refusee",
        "Windows a refusé l’installation\u{202f}: il demande peut-être des droits d’administrateur. Exportez le profil et installez-le depuis l’Explorateur (clic droit, Installer le profil).",
    ),
    (
        "installation.echec",
        "Le profil n’a pas pu être installé dans Windows. Exportez-le et installez-le depuis l’Explorateur (clic droit, Installer le profil).",
    ),
    ("bibliotheque.erreur.apercu_absent", "Aucun fichier en attente. Choisissez d’abord un fichier à importer."),
    // Bibliothèque, section Export et sauvegarde
    ("export.titre", "Export et sauvegarde"),
    ("export.pourquoi.cgats", "«\u{202f}Exporter pour le profilage…\u{202f}» écrit la mesure choisie dans un fichier que lisent les logiciels de profilage."),
    ("export.pourquoi.sauvegarde", "«\u{202f}Sauvegarder…\u{202f}» copie toute la bibliothèque dans un seul fichier, à garder en lieu sûr\u{202f}; «\u{202f}Restaurer…\u{202f}» la remet depuis une telle copie, sur ce poste ou sur un autre."),
    ("export.pourquoi.import", "«\u{202f}Importer un fichier…\u{202f}» lit un fichier de mesures CGATS et le montre avant de le ranger."),
    ("export.details.titre", "Que font ces boutons\u{202f}?"),
    (
        "export.details.cgats",
        "Format CGATS.17, un tableau par condition de mesure (M0, M1, M2) avec le spectre et les Lab de chaque plage, spectre présenté comme l’attend ArgyllCMS (SPEC_380…, en pourcentage). L’en-tête dit d’où vient la mesure (instrument, date, étalonnage), et ce qui n’est pas connu y est écrit «\u{202f}inconnue\u{202f}».",
    ),
    (
        "export.details.sauvegarde",
        "Une copie de la base de la bibliothèque (fichier SQLite), avec toutes les mesures complètes.",
    ),
    ("export.exporter", "Exporter pour le profilage…"),
    ("export.sauvegarder", "Sauvegarder…"),
    ("export.filtre.cgats", "Fichier de mesures"),
    ("export.filtre.sauvegarde", "Sauvegarde de la bibliothèque"),
    (
        "export.raison.mesure",
        "Choisissez d’abord une mesure dans la colonne de gauche.",
    ),
    ("export.fait", "Fichier écrit\u{202f}:"),
    ("export.importer", "Importer un fichier…"),
    ("export.restaurer", "Restaurer…"),
    (
        "export.restaurer.avertissement",
        "Toute la bibliothèque actuelle sera remplacée par la sauvegarde choisie. Une copie de secours datée de la bibliothèque actuelle est d’abord faite automatiquement dans son dossier.",
    ),
    (
        "export.restaurer.confirmer",
        "Choisir la sauvegarde et remplacer",
    ),
    ("export.restaurer.annuler", "Annuler"),
    ("export.restaure", "Bibliothèque restaurée depuis\u{202f}:"),
    ("import.titre", "Mesure importée"),
    ("import.apercu", "Aperçu, pas encore rangé"),
    ("import.destination", "Condition d’impression qui recevra la mesure"),
    ("import.ranger", "Ranger dans «\u{202f}{}\u{202f}»"),
    ("import.ranger.attente", "Ranger"),
    ("import.ranger.raison", "Choisissez d’abord la condition d’impression."),
    ("import.annuler", "Annuler"),
    ("import.abandonne", "Aperçu abandonné\u{202f}: rien n’a été rangé."),
    ("import.aucune_condition", "Aucune condition d’impression. Ajoutez-en une dans la colonne de gauche."),
    ("import.importee", "Importée"),
    ("import.origine", "Origine"),
    ("import.fichier", "Fichier d’origine"),
    ("import.provenance", "Provenance"),
    (
        "import.provenance.myiro",
        "myiro-libre, provenance complète",
    ),
    (
        "import.provenance.autre",
        "Autre logiciel, provenance inconnue",
    ),
    ("import.date", "Date"),
    ("import.spectre_et_lab", "spectre et Lab"),
    ("import.lab_seul", "Lab seulement"),
    ("import.spectres", "Spectres"),
    (
        "import.aucun_spectre",
        "Aucun spectre dans ce fichier\u{202f}: ce qui exige un spectre reste inconnu.",
    ),
    ("import.rangee", "Mesure rangée dans «\u{202f}{}\u{202f}»."),
    ("lecture.ponctuelle", "Couleur"),
    ("lecture.bande", "Bande"),
    ("lecture.feuille", "Feuille"),
    ("lecture.plages", "plages"),
    ("lecture.plage", "plage"),
    // Feuille de travail, au centre
    ("vide.mesurer.phrase", "Aucune mesure."),
    ("vide.mesurer.action", "Mesurer une couleur"),
    // Tâches prévues : ce qu’elles feront et quand
    ("prevu.libelle", "Prévu"),
    (
        "prevu.profiler.texte",
        "Imprimer une mire, la mesurer, puis calculer le profil ICC du papier et le ranger dans la bibliothèque, en quatre étapes.",
    ),
    ("prevu.profiler.action", "Commencer un profil"),
    (
        "prevu.profiler.raison",
        "Le calcul des profils arrive dans une prochaine étape de la livraison couleur.",
    ),
    (
        "prevu.controler.texte",
        "Mesurer un tirage et le comparer au profil de son papier, avec un verdict conforme, proche de la limite ou hors tolérance.",
    ),
    ("prevu.controler.action", "Contrôler un tirage"),
    (
        "prevu.controler.raison",
        "Le contrôle des tirages arrive avec la vérification des profils, dans la livraison couleur.",
    ),
    (
        "prevu.imprimer.texte",
        "Choisir la machine et le support, ajouter les fichiers, les placer sur le rouleau ou la feuille, puis imprimer avec le profil de la bibliothèque.",
    ),
    ("prevu.imprimer.action", "Ajouter à la file"),
    (
        "prevu.imprimer.raison",
        "L’impression arrive à la troisième étape de la livraison, après la couleur et le CMJN.",
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
    ("instrument.etat.detecte", "détecté"),
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
    ("ecran.autoriser_pare_feu", "Autoriser le FD-9 dans le pare-feu"),
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
        "probleme.aucun_fd9.cause",
        "Aucun FD-9 n’a répondu sur le réseau, alors que le pare-feu de Windows laisse passer sa réponse.",
    ),
    (
        "probleme.aucun_fd9.action",
        "Vérifiez que le FD-9 est allumé et sur le même réseau, et que FD-S2w est fermé, puis réessayez.",
    ),
    (
        "probleme.pare_feu_ferme.cause",
        "Le pare-feu de Windows bloque la réponse du FD-9\u{202f}: myiro-libre n’a pas reçu l’autorisation de l’écouter.",
    ),
    (
        "probleme.pare_feu_ferme.action",
        "Cliquez sur «\u{202f}Autoriser le FD-9 dans le pare-feu\u{202f}», puis répondez «\u{202f}Oui\u{202f}» à la fenêtre de Windows.",
    ),
    (
        "probleme.connexion_impossible.cause",
        "Le MYIRO-1 est détecté, mais il ne répond pas à la connexion. Un autre logiciel l’utilise peut-être.",
    ),
    (
        "probleme.connexion_impossible.action",
        "Fermez les autres logiciels de mesure, débranchez puis rebranchez l’instrument, et réessayez.",
    ),
    ("cartouche.inconnu", "Inconnu"),
    ("details.mesure", "Mesure"),
    ("details.condition", "Condition d’impression"),
    ("details.lecture", "Lecture"),
    ("details.instruments", "Instruments"),
    ("details.numero", "n°"),
    ("details.a_confirmer", "à confirmer"),
    ("details.valeurs", "Valeurs Lab"),
    ("details.valeurs.libelle", "Condition de mesure des valeurs"),
    ("details.plage", "Plage"),
    ("details.nom", "Nom"),
    ("details.renommer", "Renommer"),
    ("details.spectre", "Spectre"),
    ("details.spectre_sans_condition", "Spectre {} (condition inconnue)"),
    ("compte.mesure", "mesure"),
    ("compte.mesures", "mesures"),
    (
        "centre.choisir",
        "Choisissez une résolution ou une mesure dans la bibliothèque, à gauche.",
    ),
    (
        "probleme.etalonnage_echoue.cause",
        "L’étalonnage n’a pas réussi. Le MYIRO-1 n’était peut-être pas bien posé sur son capuchon.",
    ),
    (
        "probleme.etalonnage_echoue.action",
        "Posez-le bien à plat sur son capuchon, puis cliquez sur «\u{202f}Lancer l’étalonnage\u{202f}».",
    ),
    (
        "probleme.etalonnage_delai.cause",
        "Le MYIRO-1 n’a pas terminé l’étalonnage à temps.",
    ),
    (
        "probleme.etalonnage_delai.action",
        "Vérifiez qu’il est bien posé sur son capuchon, puis cliquez sur «\u{202f}Lancer l’étalonnage\u{202f}».",
    ),
    (
        "probleme.instrument_perdu.cause",
        "La liaison avec le MYIRO-1 a été perdue.",
    ),
    (
        "probleme.instrument_perdu.action",
        "Vérifiez le câble, puis cliquez sur «\u{202f}Réessayer\u{202f}» pour le reconnecter.",
    ),
    // Choix de l’instrument (ticket #49)
    ("choix.libelle", "Changer d’instrument"),
    ("choix.titre", "Instruments du poste"),
    (
        "choix.aide",
        "Un seul instrument sert à la fois. En choisir un autre ferme d’abord celui qui est en service.",
    ),
    ("choix.ligne", "{modele} ({liaison}), {etat}"),
    ("choix.liaison.usb", "USB"),
    ("choix.liaison.reseau", "réseau"),
    ("choix.etat.non_detecte", "non trouvé"),
    ("choix.etat.detecte", "détecté"),
    ("choix.etat.connecte", "connecté"),
    ("choix.etat.etalonnage_requis", "étalonnage requis"),
    ("choix.etat.etalonne", "étalonné"),
    ("choix.etat.en_attente", "en attente"),
    ("choix.etat.non_trouve", "non trouvé la dernière fois"),
    ("choix.en_service", "En service"),
    ("choix.utiliser", "Utiliser"),
    ("choix.fermer", "Fermer la liste"),
    (
        "choix.annonce.sans_choix",
        "Aucun instrument n’avait été choisi\u{202f}: le {modele}, trouvé sur ce poste, est en service. Choisissez ici celui que vous voulez utiliser.",
    ),
    (
        "choix.annonce.remplace",
        "Le {modele}, choisi la dernière fois, n’a pas été trouvé\u{202f}: le {autre} est en service à sa place.",
    ),
    ("choix.annonce.ferme", "Le {modele} a été fermé."),
    (
        "choix.annonce.ferme_repos_suppose",
        "Le {modele} a été fermé. Aucune mesure n’était en cours\u{202f}: il devrait être au repos.",
    ),
    (
        "choix.annonce.ferme_incertain",
        "Le {modele} a été fermé, mais il n’a pas confirmé son retour au repos. S’il clignote ou ne répond plus, débranchez-le puis rebranchez-le.",
    ),
    (
        "choix.annonce.ferme_echec",
        "Le {modele} n’a pas pu être fermé normalement. Débranchez-le puis rebranchez-le avant de vous en servir de nouveau.",
    ),
    (
        "refus.occupe.recherche",
        "Recherche de l’instrument en cours\u{202f}: attendez qu’elle se termine.",
    ),
    (
        "refus.occupe.etalonnage",
        "Un étalonnage est en cours\u{202f}: attendez qu’il se termine.",
    ),
    (
        "refus.occupe.mesure",
        "Une mesure est en cours\u{202f}: attendez qu’elle se termine.",
    ),
    (
        "refus.occupe.changement",
        "Changement d’instrument en cours\u{202f}: attendez qu’il se termine.",
    ),
    (
        "refus.absent",
        "Le logiciel du fabricant de cet instrument n’est pas trouvé sur ce poste.",
    ),
    (
        "choix.annonce.choix_non_retenu",
        "Le {modele} reste en service, mais ce choix n’a pas pu être gardé pour le prochain lancement.",
    ),
    ("choix.compris", "Compris"),
    (
        "choix.detail.repos_suppose",
        "Aucune mesure n’avait été lancée\u{202f}: l’instrument n’avait rien à confirmer.",
    ),
    (
        "choix.detail.repos_non_signale",
        "L’instrument n’a pas signalé à temps son retour au repos.",
    ),
    (
        "choix.detail.arret_refuse",
        "L’instrument a refusé de s’arrêter (code {code}).",
    ),
    (
        "choix.detail.liaison_perdue",
        "La liaison avec l’instrument a été perdue avant qu’il confirme son retour au repos.",
    ),
    (
        "choix.detail.deconnexion_refusee",
        "L’instrument a refusé la déconnexion (code {code})\u{202f}; la liaison a été coupée quand même.",
    ),
    (
        "choix.detail.fermeture_refusee",
        "L’instrument a refusé la fermeture\u{202f}; la liaison a été coupée quand même.",
    ),
    (
        "choix.detail.pont_muet",
        "L’instrument n’a pas répondu à la demande de fermeture\u{202f}; la liaison a été coupée quand même.",
    ),
    (
        "choix.detail.reponse_inattendue",
        "La réponse à la demande de fermeture n’était pas celle attendue\u{202f}; la liaison a été coupée quand même.",
    ),
    (
        "refus.autre",
        "Cette demande n’a pas pu aboutir. Réessayez dans un instant.",
    ),
    // Étalonnage guidé
    ("instrument.etalonner", "Étalonner"),
    (
        "raison.etalonnage",
        "Disponible une fois l’instrument étalonné.",
    ),
    ("ecran.etalonnage.etapes", "Étapes"),
    ("ecran.etalonnage.etape.blanc", "Poser sur le blanc"),
    ("ecran.etalonnage.etape.etalonnage", "Étalonnage"),
    ("ecran.etalonnage.etape.mesure", "Mesure"),
    ("ecran.etalonnage.titre", "Étalonnez le MYIRO-1."),
    (
        "ecran.etalonnage.pourquoi",
        "Avant de mesurer, le MYIRO-1 se règle sur son blanc de référence, rangé dans son capuchon. Sans ce réglage, ses mesures ne sont pas fiables. Ensuite, vous pourrez mesurer vos couleurs.",
    ),
    (
        "ecran.etalonnage.geste",
        "Posez le MYIRO-1 bien à plat sur son capuchon, puis cliquez sur «\u{202f}Lancer l’étalonnage\u{202f}».",
    ),
    ("ecran.etalonnage.lancer", "Lancer l’étalonnage"),
    ("ecran.etalonnage.annuler", "Annuler"),
    (
        "ecran.etalonnage.en_cours",
        "Étalonnage en cours. Laissez l’instrument sur son capuchon pendant quelques secondes.",
    ),
    ("ecran.etalonnage.reussi", "Le MYIRO-1 est étalonné."),
    (
        "ecran.etalonnage.reussi.suite",
        "Vous pouvez maintenant mesurer vos couleurs.",
    ),
    ("ecran.etalonnage.continuer", "Continuer"),
    (
        "ecran.etalonnage.schema",
        "Schéma\u{202f}: le MYIRO-1 posé à plat sur son capuchon, qui contient le blanc de référence.",
    ),
    ("ecran.etalonnage.schema.instrument", "MYIRO-1"),
    ("ecran.etalonnage.schema.capuchon", "Capuchon"),
    ("ecran.etalonnage.schema.blanc", "Blanc de référence"),
    // Mesure ponctuelle : feuille Mesurer ({n} est remplacé)
    ("mesurer.nom_defaut", "Couleur {n}"),
    ("mesurer.declenchement", "Mesure"),
    ("mesurer.declenchement.automatique", "Automatique"),
    ("mesurer.declenchement.manuel", "Manuelle"),
    (
        "mesurer.declenchement.non_retenu",
        "Ce choix vaut pour cette séance\u{202f}: il n’a pas pu être enregistré pour la prochaine fois.",
    ),
    (
        "mesurer.consigne.automatique",
        "Posez le MYIRO-1 sur la couleur, puis cliquez sur «\u{202f}Mesurer\u{202f}».",
    ),
    (
        "mesurer.consigne.manuel",
        "Posez le MYIRO-1 bien à plat sur la couleur, cliquez sur «\u{202f}Mesurer\u{202f}», puis appuyez sur le bouton de l’instrument.",
    ),
    ("mesurer.action", "Mesurer"),
    (
        "mesurer.en_cours.automatique",
        "Mesure en cours. Gardez le MYIRO-1 immobile sur la couleur.",
    ),
    (
        "mesurer.en_cours.manuel",
        "Appuyez sur le bouton du MYIRO-1 et gardez-le immobile jusqu’à la fin de la mesure.",
    ),
    ("mesurer.liste", "Dernières mesures"),
    ("mesurer.non_rangee", "Pas rangée"),
    (
        "mesurer.raison.aucune_condition",
        "Disponible quand une condition d’impression existe. Ajoutez-en une à gauche.",
    ),
    (
        "mesurer.raison.repos",
        "Disponible une fois l’instrument reconnecté.",
    ),
    ("mesurer.valeurs", "Valeurs"),
    (
        "mesurer.calcul",
        "Calculées par myiro-libre à partir du spectre, avec ses propres tables\u{202f}: illuminant D50, observateur 2°.",
    ),
    ("mesurer.ranger", "Ranger à nouveau"),
    ("mesurer.couleur", "Couleur à l’écran"),
    ("mesurer.approchee", "Couleur approchée à l’écran"),
    (
        "mesurer.erreur.sans_instrument.cause",
        "Aucun instrument n’est prêt à mesurer.",
    ),
    (
        "mesurer.erreur.sans_instrument.action",
        "Branchez le MYIRO-1 et attendez qu’il apparaisse en haut de la fenêtre.",
    ),
    (
        "mesurer.erreur.condition_absente.cause",
        "Cette condition d’impression n’existe plus dans la bibliothèque. Rien n’a été mesuré.",
    ),
    (
        "mesurer.erreur.condition_absente.action",
        "Choisissez-en une autre dans la liste, puis mesurez de nouveau.",
    ),
    (
        "mesurer.erreur.bibliotheque_fermee.cause",
        "La bibliothèque n’a pas pu s’ouvrir. Rien n’a été mesuré.",
    ),
    (
        "mesurer.erreur.bibliotheque_fermee.action",
        "Fermez puis relancez myiro-libre.",
    ),
    ("mesurer.lab", "L*, a*, b*"),
    ("mesurer.lch", "L*, C*, h°"),
    ("mesurer.xyz", "X, Y, Z"),
    // Couleur de référence et écart (ticket #8)
    ("mesurer.reference.titre", "Couleur de référence"),
    (
        "mesurer.reference.aucune_texte",
        "Aucune couleur de référence. Prenez une mesure comme référence pour voir l’écart des autres à elle.",
    ),
    ("mesurer.reference.designer", "Prendre comme référence"),
    ("mesurer.reference.retirer", "Retirer la référence"),
    ("mesurer.reference.marque", "Référence"),
    ("mesurer.reference.actuelle", "Couleur de référence actuelle\u{202f}: «\u{202f}{nom}\u{202f}»."),
    ("mesurer.conditions.a_confirmer", "Conditions de mesure à confirmer."),
    ("mesurer.conditions.inconnues", "Conditions de mesure inconnues."),
    (
        "mesurer.reference.est",
        "Cette mesure est la couleur de référence. Les autres mesures affichent leur écart à elle.",
    ),
    ("mesurer.reference.seuil", "Écart accepté (ΔE00)"),
    (
        "mesurer.reference.seuil_aide",
        "Laissez vide pour ne pas juger l’écart. Il est conservé avec la couleur de référence.",
    ),
    ("mesurer.reference.seuil_non_fixe", "Seuil non fixé"),
    (
        "mesurer.reference.seuil_non_fixe_texte",
        "Seuil non fixé\u{202f}: pas de verdict. Fixez l’écart accepté sur la couleur de référence.",
    ),
    (
        "mesurer.reference.non_rangee",
        "Cette mesure n’est pas rangée dans la bibliothèque, où la référence est conservée. Cliquez d’abord sur «\u{202f}Ranger à nouveau\u{202f}».",
    ),
    (
        "mesurer.reference.aucune",
        "Choisissez d’abord une couleur de référence.",
    ),
    (
        "mesurer.reference.seuil_invalide",
        "L’écart accepté doit être un nombre plus grand que zéro, par exemple 2,5.",
    ),
    ("mesurer.ecart.titre", "Écart à la référence"),
    ("mesurer.ecart.de00", "ΔE00 (CIEDE2000)"),
    ("mesurer.ecart.dc", "ΔC (saturation)"),
    ("mesurer.ecart.dh", "ΔH (teinte)"),
    ("mesurer.ecart.accepte", "Écart accepté"),
    ("mesurer.ecart.comparaison", "En haut la couleur de référence, en bas cette mesure"),
    ("mesurer.liste.ecart", "ΔE00 (CIEDE2000)"),
    ("mesurer.liste.verdict", "Verdict"),
    ("verdict.conforme", "Conforme"),
    ("verdict.proche_de_la_limite", "Proche de la limite"),
    ("verdict.hors_tolerance", "Hors tolérance"),
    ("verdict.inconnu", "Écart inconnu"),
    ("verdict.non_comparable", "Conditions différentes"),
    ("mesurer.verdict.conforme", "L’écart reste dans la limite fixée."),
    (
        "mesurer.verdict.proche_de_la_limite",
        "L’écart reste dans la limite fixée, mais s’en approche.",
    ),
    ("mesurer.verdict.hors_tolerance", "L’écart dépasse la limite fixée."),
    (
        "mesurer.verdict.inconnu",
        "L’écart ne peut pas être calculé\u{202f}: les valeurs d’une des deux mesures sont inconnues.",
    ),
    (
        "mesurer.ecart.condition_differente",
        "Cette mesure et la couleur de référence n’ont pas la même condition de mesure\u{202f}: elles ne se comparent pas, ni écart ni verdict.",
    ),
    (
        "mesurer.ecart.non_confirmee",
        "La condition de mesure n’est pas confirmée pour les deux mesures\u{202f}: cet écart est à prendre avec prudence.",
    ),
    ("mesurer.ecart.calcul_titre", "Comment l’écart est calculé"),
    (
        "mesurer.ecart.calcul",
        "ΔE00\u{202f}: formule CIEDE2000, facteurs k_L = k_C = k_H = 1.\nΔC et ΔH\u{202f}: écarts de chroma et de teinte CIELAB, signés (positifs\u{202f}: mesure plus saturée, teinte tournée dans le sens direct).\nLab D50, observateur 2°, calculés par myiro-libre à partir des spectres de même condition de mesure.\nProche de la limite\u{202f}: à partir de 80\u{202f}% de l’écart accepté, un choix de myiro-libre et non d’une norme. Le verdict porte sur l’écart tel qu’il est écrit, à deux décimales.\nConditions de mesure différentes\u{202f}: ni écart ni verdict.",
    ),
    ("cartouche.micrologiciel", "Micrologiciel"),
    ("cartouche.date", "Date"),
    (
        "mesurer.erreur.rangement",
        "La bibliothèque n’a pas pu ranger cette mesure. Elle reste affichée ici jusqu’à la fermeture de myiro-libre.",
    ),
    // Mesure ponctuelle : problèmes
    (
        "probleme.mesure_echouee.cause",
        "La mesure n’a pas réussi. Le MYIRO-1 n’était peut-être pas bien à plat sur la couleur.",
    ),
    (
        "probleme.mesure_echouee.action",
        "Posez-le bien à plat, cliquez sur «\u{202f}Mesurer\u{202f}», puis appuyez sur son bouton.",
    ),
    (
        "probleme.mesure_delai.cause",
        "Le bouton du MYIRO-1 n’a pas été appuyé à temps.",
    ),
    (
        "probleme.mesure_delai.action",
        "Cliquez de nouveau sur «\u{202f}Mesurer\u{202f}», puis appuyez sur le bouton de l’instrument dans les deux minutes.",
    ),
    (
        "probleme.mesure_delai_automatique.cause",
        "Le MYIRO-1 n’a pas terminé la mesure à temps.",
    ),
    (
        "probleme.mesure_delai_automatique.action",
        "Vérifiez qu’il est bien posé à plat sur la couleur, puis cliquez de nouveau sur «\u{202f}Mesurer\u{202f}».",
    ),
    (
        "probleme.declenchement_refuse.cause",
        "Le MYIRO-1 n’a pas accepté de mesurer sans qu’on appuie sur son bouton.",
    ),
    (
        "probleme.declenchement_refuse.action",
        "Choisissez «\u{202f}Mesure\u{202f}: manuelle\u{202f}», cliquez sur «\u{202f}Mesurer\u{202f}», puis appuyez sur le bouton de l’instrument.",
    ),
    (
        "probleme.etalonnage_a_refaire.cause",
        "Le MYIRO-1 demande un nouvel étalonnage.",
    ),
    (
        "probleme.etalonnage_a_refaire.action",
        "Cliquez sur «\u{202f}Étalonner\u{202f}», en haut, puis mesurez de nouveau.",
    ),
    (
        "probleme.repos_incertain.cause",
        "Le MYIRO-1 n’a pas confirmé la fin de la mesure. La mesure est gardée.",
    ),
    (
        "probleme.repos_incertain.action",
        "Pour mesurer de nouveau, cliquez sur «\u{202f}Réessayer\u{202f}»\u{202f}: l’instrument sera reconnecté, puis à étalonner.",
    ),
    // Vérification d'un profil (ticket #41)
    (
        "profil.erreur.outil_manquant",
        "Un outil d’ArgyllCMS manque dans le dossier argyll à côté de l’application. Remplacez ce dossier par une copie complète d’ArgyllCMS 3.5.0, puis relancez l’application.",
    ),
    ("verif.titre", "Vérifier le profil"),
    ("verif.profil", "Profil à vérifier"),
    ("verif.mesure", "Mesure de la seconde mire"),
    ("verif.condition", "Condition de mesure"),
    ("verif.tolerances", "Tolérances"),
    ("verif.fiche", "Fiche d’impression"),
    ("verif.fiche.impression", "Impression de la seconde mire"),
    (
        "verif.fiche.impression.a_travers_le_profil",
        "À travers ce profil, depuis des couleurs sRVB, en colorimétrique relatif",
    ),
    (
        "verif.fiche.impression.sans_gestion",
        "Sans gestion des couleurs",
    ),
    ("verif.fiche.logiciel", "Logiciel d’impression"),
    ("verif.fiche.support_pilote", "Support choisi dans le pilote"),
    ("verif.fiche.qualite", "Qualité et résolution"),
    ("verif.fiche.encres", "Encres"),
    ("verif.fiche.remarques", "Remarques"),
    ("verif.fiche.confirmer", "Je confirme ces réglages"),
    ("verif.lancer", "Vérifier"),
    ("verif.en_cours", "Vérification en cours…"),
    ("verif.avance", "Avancé"),
    ("verif.fidelite", "Fidélité aux couleurs demandées"),
    ("verif.prediction", "Accord avec la prédiction du profil"),
    ("verif.hors_gamut", "Couleurs hors gamut"),
    (
        "verif.inconnu.sans_demande",
        "inconnue\u{202f}: la mire a été imprimée sans gestion des couleurs, aucune couleur n’était demandée",
    ),
    (
        "verif.inconnu.aucune_plage_dans_le_gamut",
        "inconnue\u{202f}: aucune couleur demandée n’est dans le gamut du profil",
    ),
    ("verif.inconnu.aucune_plage", "aucune plage"),
    ("verif.inconnu", "inconnu"),
    ("verif.etat.non_verifie", "À vérifier"),
    ("verif.etat.accepte", "Accepté selon {tolerances}"),
    ("verif.etat.hors_tolerance", "Hors tolérance"),
    ("verif.etat.incomplet", "Vérification incomplète"),
    ("verif.critere.moyenne", "ΔE00 moyen"),
    ("verif.critere.p95", "95\u{202f}% des plages"),
    ("verif.critere.max", "ΔE00 maximum"),
    ("verif.plages", "Plages"),
    ("verif.pires", "Plages les plus éloignées"),
    ("verif.respecte", "respecté"),
    ("verif.depasse", "dépassé"),
    ("verif.calculee", "calculée par ArgyllCMS"),
    ("verif.demande", "Couleur demandée"),
    ("verif.appareil", "Valeur après transformation"),
    ("verif.predite", "Prédiction du profil"),
    ("verif.mesuree", "Mesure"),
    (
        "verif.memes_couleurs",
        "Mêmes couleurs que la mire du profil\u{202f}: contrôle moins exigeant qu’une mire différente.",
    ),
    (
        "verif.regle",
        "Un seuil dépassé donne «\u{202f}hors tolérance\u{202f}». Sinon, un critère qui ne peut pas être calculé donne «\u{202f}vérification incomplète\u{202f}». Sinon, le profil est accepté.",
    ),
    ("verif.recalculer", "Recalculer avec d’autres tolérances"),
    ("verif.exporter_rapport", "Exporter le rapport"),
    (
        "verif.exporter_profil",
        "Exporter le profil avec son rapport",
    ),
    ("verif.historique", "Vérifications de ce profil"),
    ("verif.derniere", "Dernière vérification"),
    (
        "verif.indisponible.sans_profil",
        "Aucun profil RVB actif à vérifier dans cette résolution.",
    ),
    (
        "verif.refus.fiche",
        "La fiche d’impression n’est pas confirmée. Vérifiez les réglages, puis cochez la case de confirmation.",
    ),
    (
        "verif.refus.mesure_du_profil",
        "Cette mesure a servi à calculer le profil\u{202f}: elle ne peut pas le vérifier. Choisissez la mesure d’une seconde mire.",
    ),
    (
        "verif.refus.cmjn",
        "La vérification d’un profil CMJN n’est pas encore prévue. Choisissez un profil RVB.",
    ),
    (
        "verif.refus.vide",
        "Cette mesure ne contient aucune plage. Importez un autre fichier.",
    ),
    (
        "verif.refus.tailles",
        "ArgyllCMS n’a pas rendu une valeur pour chaque plage. Rien n’a été rangé. Relancez la vérification.",
    ),
    (
        "verif.refus.valeur",
        "Une plage de cette mesure ou une valeur du profil est inutilisable. Rien n’a été rangé. Vérifiez la mesure, puis importez-la à nouveau.",
    ),
    (
        "verif.erreur.calcul",
        "ArgyllCMS n’a pas pu interroger le profil. Rien n’a été rangé. Vérifiez le profil, puis relancez la vérification.",
    ),
    ("verif.tolerances.nom", "Nom du jeu"),
    ("verif.tolerances.seuil_gamut", "Seuil hors gamut (ΔE00)"),
    ("verif.tolerances.enregistrer", "Enregistrer"),
    (
        "verif.tolerances.proposition",
        "Proposition, à confirmer\u{202f}: aucune valeur n’est imposée.",
    ),
    (
        "verif.tolerances.enregistre",
        "{jeu} enregistré\u{202f}; les vérifications déjà faites ne changent pas.",
    ),
    (
        "verif.tolerances.aucun_seuil",
        "Ce jeu n’a aucun seuil. Renseignez au moins un seuil, puis enregistrez.",
    ),
    (
        "verif.tolerances.seuil_invalide",
        "Un seuil n’est pas un nombre plus grand que zéro. Corrigez-le, puis enregistrez.",
    ),
    (
        "verif.tolerances.aucun",
        "Aucun jeu de tolérances enregistré. Enregistrez-en un avant de vérifier.",
    ),
    // Rapport de vérification (HTML)
    ("rapport.titre", "Rapport de vérification du profil"),
    ("rapport.sans_seuil", "aucun seuil"),
    ("rapport.resultats", "Résultats"),
    ("rapport.controles", "Contrôles un par un"),
    ("rapport.famille", "Famille"),
    ("rapport.critere", "Critère"),
    ("rapport.limite", "Seuil"),
    ("rapport.valeur", "Valeur trouvée"),
    ("rapport.resultat", "Résultat"),
    ("rapport.toutes_plages", "Toutes les plages"),
    ("rapport.plage", "Plage"),
    ("rapport.rvb", "RVB de la mire (0 à 255)"),
    ("rapport.ecart_fidelite", "ΔE00 fidélité"),
    ("rapport.ecart_prediction", "ΔE00 prédiction"),
    ("rapport.ecart_gamut", "ΔE00 au gamut"),
    ("rapport.recalcul", "Recalcul sans nouvelle mesure de la vérification du"),
    ("rapport.date", "Date"),
    ("rapport.profil", "Profil vérifié"),
    ("rapport.nom", "Nom"),
    ("rapport.empreinte", "Empreinte SHA-256"),
    ("rapport.recette", "Recette du calcul"),
    ("rapport.argyll", "Version d’ArgyllCMS"),
    ("rapport.source", "Source de la mesure"),
    ("rapport.fichier", "Fichier importé"),
    ("rapport.instrument", "Instrument déclaré par le fichier"),
    ("rapport.supposee", "supposée d’après la convention du fabricant"),
    ("rapport.commandes", "Commandes ArgyllCMS"),
];

const ANGLAIS: Catalogue = &[
    // Top bar
    ("nav.libelle", "Tasks"),
    ("tache.mesurer", "Measure"),
    ("tache.controler", "Check"),
    ("tache.profiler", "Profile"),
    ("tache.bibliotheque", "Library"),
    ("tache.imprimer", "Print"),
    ("marque.sous_titre", "colour and print station"),
    ("instrument.libelle", "Instrument"),
    ("instrument.aucun", "No instrument"),
    ("theme.libelle", "Theme"),
    ("theme.clair", "Light"),
    ("theme.sombre", "Dark"),
    ("langue.libelle", "Language"),
    // Library, left
    ("bibliotheque.recherche", "Search"),
    ("bibliotheque.recherche_aide", "Printer, paper or resolution"),
    (
        "bibliotheque.aucun_resultat",
        "Nothing matches this search.",
    ),
    ("bibliotheque.demonstration", "Demonstration, sample data"),
    ("bibliotheque.mesures", "Measurements"),
    ("bibliotheque.erreur.nom_vide", "Give it a name."),
    (
        "bibliotheque.erreur.nom_pris",
        "Another printing condition already has this name.",
    ),
    (
        "bibliotheque.erreur.autre",
        "The library could not do this.",
    ),
    (
        "bibliotheque.erreur.ouverture",
        "The library could not open.",
    ),
    (
        "bibliotheque.erreur.sauvegarde_invalide",
        "This file is not a usable backup. The library has not changed.",
    ),
    (
        "bibliotheque.erreur.cgats_illisible",
        "This file could not be read as CGATS, or it does not state its measurement condition. Nothing was guessed.",
    ),
    (
        "bibliotheque.erreur.cgats_trop_gros",
        "This file is too large for a measurement file (over 16 MiB). Nothing was imported.",
    ),
    ("bibliotheque.erreur.export", "The file could not be written."),
    // Fleet: printers, papers, resolutions and profiles
    (
        "parc.vide",
        "No printer yet. Add the workshop’s first printer, then its papers.",
    ),
    ("parc.ajouter_imprimante", "Add a printer"),
    ("parc.titre", "Printers and papers"),
    ("parc.non_classees", "Unfiled"),
    ("parc.ajouter_papier", "Add a paper"),
    ("parc.ajouter_resolution", "Add a resolution"),
    ("parc.ajout.imprimante", "New printer"),
    ("parc.ajout.papier", "New paper for {}"),
    ("parc.ajout.resolution", "New resolution for {}"),
    ("parc.champ.imprimante", "Printer name"),
    ("parc.champ.papier", "Paper name"),
    ("parc.champ.resolution", "Print resolution"),
    ("parc.ajouter", "Add"),
    ("parc.annuler", "Cancel"),
    ("parc.compte.papier", "paper"),
    ("parc.compte.papiers", "papers"),
    ("parc.age.aucun", "none"),
    ("parc.age.ancien", "{} · old"),
    ("parc.age.plus_d_un_an", "over a year old"),
    ("parc.verdict.contexte", "Current profile"),
    ("parc.verdict.verifie", "Profile verified"),
    ("parc.verdict.non_verifie", "Profile to be verified"),
    ("parc.verdict.calcule_a_verifier", "Profile computed, to be verified"),
    ("parc.verdict.importe_a_verifier", "Profile imported, to be verified"),
    ("parc.verdict.hors", "Profile out of tolerance"),
    ("parc.verdict.incomplet", "Verification incomplete"),
    ("parc.verdict.aucun", "No profile"),
    (
        "parc.verdict.aucun_contexte",
        "Store an existing profile for this resolution",
    ),
    ("parc.espace.rvb", "RGB"),
    ("parc.espace.cmjn", "CMYK"),
    ("parc.installer", "Install in Windows"),
    (
        "parc.installer.raison",
        "No profile for this resolution. Store an existing profile first.",
    ),
    ("parc.exporter", "Export"),
    ("parc.ranger", "Store an existing profile"),
    ("parc.range", "Profile stored. It is now the current profile."),
    (
        "parc.en_cours",
        "Action in progress on this profile. The buttons come back as soon as it is done.",
    ),
    (
        "parc.deja_installe",
        "This profile is already installed in Windows, unchanged.",
    ),
    ("parc.filtre.icc", "ICC profile"),
    ("parc.historique", "History"),
    ("parc.historique.actuel", "current"),
    ("parc.historique.remplace", "replaced"),
    ("parc.historique.vide", "No profile yet."),
    ("parc.etat.non_verifie", "to be verified"),
    ("parc.etat.accepte", "verified"),
    ("parc.etat.hors_tolerance", "out of tolerance"),
    ("parc.etat.incomplet", "incomplete"),
    ("parc.mesures.vide", "No measurement for this resolution."),
    ("parc.provenance", "Provenance"),
    ("parc.origine", "Origin"),
    ("parc.origine.profil_existant", "existing profile"),
    ("parc.origine.calcul", "computed by myiro-libre"),
    ("parc.nom", "File"),
    ("parc.empreinte", "Fingerprint"),
    ("parc.instrument", "Instrument"),
    ("parc.mire", "Chart"),
    ("parc.inconnu", "unknown"),
    ("parc.inconnue", "unknown"),
    ("parc.fiche", "Print settings"),
    ("parc.fiche.reglages", "Driver settings"),
    ("parc.windows", "Windows"),
    ("parc.windows.etat", "Installation"),
    ("parc.windows.non_installe", "not installed"),
    ("parc.windows.sans_profil", "no profile"),
    (
        "parc.erreur.imprimante_inconnue",
        "This printer is no longer in the library.",
    ),
    (
        "parc.erreur.papier_inconnu",
        "This paper is no longer in the library.",
    ),
    (
        "parc.erreur.reglage_inconnu",
        "This resolution is no longer in the library.",
    ),
    (
        "parc.erreur.profil_inconnu",
        "This profile is no longer in the library.",
    ),
    (
        "parc.erreur.profil_illisible",
        "This file is not an RGB or CMYK ICC profile. Choose a printer .icc or .icm file.",
    ),
    (
        "parc.erreur.fichier_illisible",
        "The file could not be read. Check that it still exists and is not open elsewhere.",
    ),
    // Computing a profile (ticket #40)
    (
        "profil.titre",
        "Compute a profile",
    ),
    (
        "profil.source",
        "Measurement",
    ),
    (
        "profil.condition",
        "Measurement condition",
    ),
    (
        "profil.recette",
        "Calculation recipe",
    ),
    (
        "profil.recette.rapide",
        "Fast: for a trial, the shortest (about 1 min for 2,520 patches)",
    ),
    (
        "profil.recette.standard",
        "Standard: advised for production (about 3 min for 2,520 patches)",
    ),
    (
        "profil.recette.approfondi",
        "Thorough: being tuned, not available yet",
    ),
    (
        "profil.avance",
        "Advanced: equivalent command",
    ),
    (
        "profil.calculer",
        "Compute the profile",
    ),
    (
        "profil.en_cours",
        "Computing the profile. This may take several minutes.",
    ),
    (
        "profil.etape.preparation",
        "Preparing the measurements",
    ),
    (
        "profil.etape.profil",
        "Computing the profile (colprof): curves",
    ),
    (
        "profil.etape.table",
        "Computing the profile (colprof): main table",
    ),
    (
        "profil.etape.gamut",
        "Computing the profile (colprof): gamut mapping",
    ),
    (
        "profil.etape.inverses",
        "Computing the profile (colprof): inverse tables",
    ),
    (
        "profil.etape.limite",
        "Computing the profile (colprof): gamut boundary",
    ),
    (
        "profil.progression",
        "Calculation progress",
    ),
    (
        "profil.progression.etape",
        "Computing the profile, step {n} of {total}",
    ),
    (
        "profil.estimation.restante",
        "about {duree} left (estimated)",
    ),
    (
        "profil.estimation.depassee",
        "longer than the last calculation with this recipe (estimated)",
    ),
    (
        "profil.etape.controle",
        "Checking accuracy (profcheck)",
    ),
    (
        "profil.ecoule",
        "elapsed time: {duree}",
    ),
    (
        "profil.argyll.ligne",
        "Last line from ArgyllCMS: {ligne}",
    ),
    (
        "profil.argyll.pourcentage",
        "Last line from ArgyllCMS: {ligne} — {pourcentage}% of this part",
    ),
    (
        "profil.fait",
        "Profile computed. Mean difference {moyenne} ΔE00, maximum {max}. It is still to be verified.",
    ),
    (
        "profil.supposee",
        "assumed",
    ),
    (
        "profil.refus.condition_absente",
        "This measurement does not contain this measurement condition. Choose a condition offered in the list.",
    ),
    (
        "profil.refus.rvb_inconnus",
        "This measurement does not say which RGB values were printed. Import a CGATS export from FD-S2w or MYIROtools that contains the RGB columns.",
    ),
    (
        "profil.refus.lab_inconnu",
        "A patch of this measurement has no Lab value for this condition. Choose another condition, or measure the chart again.",
    ),
    (
        "profil.refus.rvb_hors_echelle",
        "A patch of this measurement has an RGB value outside the expected scale. Check the exported file, then import it again.",
    ),
    (
        "profil.refus.vide",
        "This measurement contains no patch. Import another file.",
    ),
    (
        "profil.refus.aucune_condition",
        "No measurement condition of this file has all its patches in Lab. Import a complete export from FD-S2w or MYIROtools.",
    ),
    (
        "profil.erreur.argyll_introuvable",
        "ArgyllCMS 3.5.0 cannot be found. Place it in the argyll folder next to the application, then restart the application.",
    ),
    (
        "profil.erreur.argyll_version",
        "ArgyllCMS {version} was found, but only version 3.5.0 is validated. Place ArgyllCMS 3.5.0 in the argyll folder next to the application, then restart the application.",
    ),
    (
        "profil.erreur.argyll_illisible",
        "ArgyllCMS does not answer as expected: its version could not be read. Replace the argyll folder next to the application with a complete copy of ArgyllCMS 3.5.0, then restart the application.",
    ),
    (
        "profil.erreur.calcul",
        "Computing the profile failed in ArgyllCMS. The current profile has not changed. Check the measurement, then try again.",
    ),
    (
        "profil.erreur.recette_en_reglage",
        "The Thorough recipe is being tuned. Choose Fast or Standard.",
    ),
    (
        "profil.erreur.interrompu",
        "The computation was interrupted before the end (ArgyllCMS program stopped, or computer put to sleep). The current profile has not changed. Start the computation again.",
    ),
    (
        "profil.erreur.controle",
        "Checking the computed profile failed in ArgyllCMS. Nothing was stored, the current profile has not changed. Try again.",
    ),
    (
        "profil.erreur.fichier",
        "A working file of the calculation could not be read or written. Nothing was stored. Try again, then restart the computer if the error comes back.",
    ),
    (
        "profil.erreur.mesure_ailleurs",
        "This measurement is stored in another resolution. Choose a measurement of this resolution.",
    ),
    ("profil.erreur.cause", "ArgyllCMS message: {cause}"),
    ("profil.recette.nom.rapide", "Fast"),
    ("profil.recette.nom.standard", "Standard"),
    ("profil.recette.nom.approfondi", "Thorough"),
    (
        "profil.calculer.raison",
        "No measurement of this resolution can be used to compute a profile. The reason is given under each measurement.",
    ),
    (
        "profil.sans_mesure",
        "First import a CGATS export from FD-S2w or MYIROtools into this resolution.",
    ),
    ("profil.version", "ArgyllCMS"),
    ("profil.journal", "ArgyllCMS log"),
    ("profil.rvb", "RGB values"),
    ("profil.rvb.supposee", "0 to 255 scale assumed"),
    ("profil.avance.provenance", "Advanced: command and log"),
    (
        "parc.erreur.inattendue",
        "The action stopped on an unexpected error. Choose this resolution again in the library, then try again.",
    ),
    ("profil.precision", "Calculation accuracy"),
    ("profil.duree", "Calculation time"),
    (
        "profil.precision.valeur",
        "{moyenne} on average, {max} at most (ΔE00)",
    ),
    // Measurement consistency check (ticket #1)
    (
        "profil.refus.incoherent",
        "The measurements do not match the patches of the chart: the profile would be wrong. Patches involved: {plages}. Read the chart again, check the sheet orientation and the reading order, and that this file belongs to this chart.",
    ),
    ("profil.coherence.autres", "and {n} others"),
    ("profil.coherence.autre", "and 1 other"),
    (
        "profil.coherence.decalage",
        "The measurements seem shifted by {n} patches.",
    ),
    (
        "profil.coherence.decalage_une",
        "The measurements seem shifted by one patch.",
    ),
    ("profil.coherence.criteres", "Checks that flag them: {criteres}."),
    (
        "profil.coherence.critere.degrade_gris",
        "grey ramp, dark to light",
    ),
    ("profil.coherence.critere.blanc_noir", "paper white and black"),
    ("profil.coherence.critere.doublons", "repeated patches"),
    ("profil.coherence.critere.voisines", "neighbouring patches"),
    ("profil.coherence", "Measurements checked"),
    ("profil.coherence.accepte", "consistent"),
    ("profil.coherence.non_fait", "Checks not done"),
    (
        "profil.coherence.non_controle",
        "no, profile calculated before this check",
    ),
    // Installing in Windows
    ("installation.installe", "Installed in Windows"),
    (
        "installation.nom_invalide",
        "This profile’s name does not suit Windows. It must be a plain file name ending in .icc or .icm.",
    ),
    (
        "installation.deja_present",
        "Another profile with the same name is already installed in Windows: rename this profile or remove the other one.",
    ),
    (
        "installation.refusee",
        "Windows refused the installation: it may require administrator rights. Export the profile and install it from File Explorer (right-click, Install Profile).",
    ),
    (
        "installation.echec",
        "The profile could not be installed in Windows. Export it and install it from File Explorer (right-click, Install Profile).",
    ),
    ("bibliotheque.erreur.apercu_absent", "No file waiting. First choose a file to import."),
    ("export.titre", "Export and backup"),
    ("export.pourquoi.cgats", "“Export for profiling…” writes the chosen measurement to a file that profiling software reads."),
    ("export.pourquoi.sauvegarde", "“Back up…” copies the whole library to a single file to keep somewhere safe; “Restore…” brings it back from such a copy, on this computer or another."),
    ("export.pourquoi.import", "“Import a file…” reads a CGATS measurement file and shows it before storing it."),
    ("export.details.titre", "What do these buttons do?"),
    (
        "export.details.cgats",
        "CGATS.17 format, one table per measurement condition (M0, M1, M2) with the spectrum and Lab values of each patch, spectrum laid out as ArgyllCMS expects (SPEC_380…, in percent). The header says where the measurement comes from (instrument, date, calibration), and anything unknown is written “inconnue”.",
    ),
    (
        "export.details.sauvegarde",
        "A copy of the library database (SQLite file), with every measurement in full.",
    ),
    ("export.exporter", "Export for profiling…"),
    ("export.sauvegarder", "Back up…"),
    ("export.filtre.cgats", "Measurement file"),
    ("export.filtre.sauvegarde", "Library backup"),
    (
        "export.raison.mesure",
        "First choose a measurement in the left column.",
    ),
    ("export.fait", "File written:"),
    ("export.importer", "Import a file…"),
    ("export.restaurer", "Restore…"),
    (
        "export.restaurer.avertissement",
        "The whole current library will be replaced by the chosen backup. A dated safety copy of the current library is made automatically in its folder.",
    ),
    (
        "export.restaurer.confirmer",
        "Choose the backup and replace",
    ),
    ("export.restaurer.annuler", "Cancel"),
    ("export.restaure", "Library restored from:"),
    ("import.titre", "Imported measurement"),
    ("import.apercu", "Preview, not stored yet"),
    ("import.destination", "Printing condition that will receive the measurement"),
    ("import.ranger", "Store in “{}”"),
    ("import.ranger.attente", "Store"),
    ("import.ranger.raison", "First choose the printing condition."),
    ("import.annuler", "Cancel"),
    ("import.abandonne", "Preview abandoned: nothing was stored."),
    ("import.aucune_condition", "No printing condition yet. Add one in the left column."),
    ("import.importee", "Imported"),
    ("import.origine", "Origin"),
    ("import.fichier", "Original file"),
    ("import.provenance", "Provenance"),
    ("import.provenance.myiro", "myiro-libre, full provenance"),
    ("import.provenance.autre", "Other software, provenance unknown"),
    ("import.date", "Date"),
    ("import.spectre_et_lab", "spectrum and Lab"),
    ("import.lab_seul", "Lab only"),
    ("import.spectres", "Spectra"),
    (
        "import.aucun_spectre",
        "No spectrum in this file: anything that needs a spectrum stays unknown.",
    ),
    ("import.rangee", "Measurement stored in “{}”."),
    ("lecture.ponctuelle", "Colour"),
    ("lecture.bande", "Strip"),
    ("lecture.feuille", "Sheet"),
    ("lecture.plages", "patches"),
    ("lecture.plage", "patch"),
    // Work sheet, centre
    ("vide.mesurer.phrase", "No measurement yet."),
    ("vide.mesurer.action", "Measure a colour"),
    // Planned tasks: what they will do and when
    ("prevu.libelle", "Planned"),
    (
        "prevu.profiler.texte",
        "Print a chart, measure it, then compute the paper’s ICC profile and store it in the library, in four steps.",
    ),
    ("prevu.profiler.action", "Start a profile"),
    (
        "prevu.profiler.raison",
        "Profile computation arrives in a coming step of the colour release.",
    ),
    (
        "prevu.controler.texte",
        "Measure a print and compare it with its paper’s profile, with a verdict: within tolerance, near the limit or out of tolerance.",
    ),
    ("prevu.controler.action", "Check a print"),
    (
        "prevu.controler.raison",
        "Print checks arrive with profile verification, in the colour release.",
    ),
    (
        "prevu.imprimer.texte",
        "Choose the machine and media, add the files, place them on the roll or sheet, then print with the profile from the library.",
    ),
    ("prevu.imprimer.action", "Add to the queue"),
    (
        "prevu.imprimer.raison",
        "Printing arrives in the third step of the release, after colour and CMYK.",
    ),
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
    ("instrument.etat.detecte", "detected"),
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
    ("ecran.autoriser_pare_feu", "Allow the FD-9 in the firewall"),
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
        "probleme.aucun_fd9.cause",
        "No FD-9 answered on the network, although the Windows firewall lets its reply through.",
    ),
    (
        "probleme.aucun_fd9.action",
        "Check that the FD-9 is on and on the same network, and that FD-S2w is closed, then try again.",
    ),
    (
        "probleme.pare_feu_ferme.cause",
        "The Windows firewall blocks the FD-9 reply: myiro-libre was not allowed to listen for it.",
    ),
    (
        "probleme.pare_feu_ferme.action",
        "Click “Allow the FD-9 in the firewall”, then answer “Yes” in the Windows window.",
    ),
    (
        "probleme.connexion_impossible.cause",
        "The MYIRO-1 is detected but does not answer the connection. Another program may be using it.",
    ),
    (
        "probleme.connexion_impossible.action",
        "Close other measuring software, unplug and replug the instrument, then try again.",
    ),
    ("cartouche.inconnu", "Unknown"),
    ("details.mesure", "Measurement"),
    ("details.condition", "Printing condition"),
    ("details.lecture", "Reading"),
    ("details.instruments", "Instruments"),
    ("details.numero", "No."),
    ("details.a_confirmer", "to be confirmed"),
    ("details.valeurs", "Lab values"),
    (
        "details.valeurs.libelle",
        "Measurement condition of the values",
    ),
    ("details.plage", "Patch"),
    ("details.nom", "Name"),
    ("details.renommer", "Rename"),
    ("details.spectre", "Spectrum"),
    ("details.spectre_sans_condition", "Spectrum {} (unknown condition)"),
    ("compte.mesure", "measurement"),
    ("compte.mesures", "measurements"),
    (
        "centre.choisir",
        "Choose a resolution or a measurement in the library, on the left.",
    ),
    (
        "probleme.etalonnage_echoue.cause",
        "Calibration failed. The MYIRO-1 may not have been sitting properly on its cap.",
    ),
    (
        "probleme.etalonnage_echoue.action",
        "Set it flat on its cap, then click “Start calibration”.",
    ),
    (
        "probleme.etalonnage_delai.cause",
        "The MYIRO-1 did not finish calibrating in time.",
    ),
    (
        "probleme.etalonnage_delai.action",
        "Check that it is sitting properly on its cap, then click “Start calibration”.",
    ),
    (
        "probleme.instrument_perdu.cause",
        "The connection with the MYIRO-1 was lost.",
    ),
    (
        "probleme.instrument_perdu.action",
        "Check the cable, then click “Try again” to reconnect it.",
    ),
    // Instrument choice (ticket #49)
    ("choix.libelle", "Change instrument"),
    ("choix.titre", "Instruments on this computer"),
    (
        "choix.aide",
        "Only one instrument works at a time. Choosing another first closes the one in use.",
    ),
    ("choix.ligne", "{modele} ({liaison}), {etat}"),
    ("choix.liaison.usb", "USB"),
    ("choix.liaison.reseau", "network"),
    ("choix.etat.non_detecte", "not found"),
    ("choix.etat.detecte", "detected"),
    ("choix.etat.connecte", "connected"),
    ("choix.etat.etalonnage_requis", "calibration required"),
    ("choix.etat.etalonne", "calibrated"),
    ("choix.etat.en_attente", "on standby"),
    ("choix.etat.non_trouve", "not found last time"),
    ("choix.en_service", "In use"),
    ("choix.utiliser", "Use"),
    ("choix.fermer", "Close the list"),
    (
        "choix.annonce.sans_choix",
        "No instrument had been chosen: the {modele}, found on this computer, is in use. Choose here the one you want to use.",
    ),
    (
        "choix.annonce.remplace",
        "The {modele}, chosen last time, was not found: the {autre} is in use instead.",
    ),
    ("choix.annonce.ferme", "The {modele} has been closed."),
    (
        "choix.annonce.ferme_repos_suppose",
        "The {modele} has been closed. No measurement was running: it should be at rest.",
    ),
    (
        "choix.annonce.ferme_incertain",
        "The {modele} has been closed, but it did not confirm it is back at rest. If it blinks or stops responding, unplug it and plug it back in.",
    ),
    (
        "choix.annonce.ferme_echec",
        "The {modele} could not be closed normally. Unplug it and plug it back in before using it again.",
    ),
    (
        "refus.occupe.recherche",
        "Looking for the instrument: wait until it is done.",
    ),
    (
        "refus.occupe.etalonnage",
        "A calibration is running: wait until it is done.",
    ),
    (
        "refus.occupe.mesure",
        "A measurement is running: wait until it is done.",
    ),
    (
        "refus.occupe.changement",
        "Changing instrument: wait until it is done.",
    ),
    (
        "refus.absent",
        "The manufacturer software for this instrument is not found on this computer.",
    ),
    (
        "choix.annonce.choix_non_retenu",
        "The {modele} stays in use, but this choice could not be kept for the next launch.",
    ),
    ("choix.compris", "Got it"),
    (
        "choix.detail.repos_suppose",
        "No measurement had been started: the instrument had nothing to confirm.",
    ),
    (
        "choix.detail.repos_non_signale",
        "The instrument did not report in time that it was back at rest.",
    ),
    (
        "choix.detail.arret_refuse",
        "The instrument refused to stop (code {code}).",
    ),
    (
        "choix.detail.liaison_perdue",
        "The connection with the instrument was lost before it confirmed it was back at rest.",
    ),
    (
        "choix.detail.deconnexion_refusee",
        "The instrument refused to disconnect (code {code}); the connection was cut anyway.",
    ),
    (
        "choix.detail.fermeture_refusee",
        "The instrument refused to close; the connection was cut anyway.",
    ),
    (
        "choix.detail.pont_muet",
        "The instrument did not answer the request to close; the connection was cut anyway.",
    ),
    (
        "choix.detail.reponse_inattendue",
        "The answer to the request to close was not the expected one; the connection was cut anyway.",
    ),
    (
        "refus.autre",
        "This request could not be completed. Try again in a moment.",
    ),
    // Guided calibration
    ("instrument.etalonner", "Calibrate"),
    (
        "raison.etalonnage",
        "Available once the instrument is calibrated.",
    ),
    ("ecran.etalonnage.etapes", "Steps"),
    ("ecran.etalonnage.etape.blanc", "Set on the white"),
    ("ecran.etalonnage.etape.etalonnage", "Calibration"),
    ("ecran.etalonnage.etape.mesure", "Measurement"),
    ("ecran.etalonnage.titre", "Calibrate the MYIRO-1."),
    (
        "ecran.etalonnage.pourquoi",
        "Before measuring, the MYIRO-1 sets itself against its reference white, kept in its cap. Without this, its measurements are not reliable. Then you can measure your colours.",
    ),
    (
        "ecran.etalonnage.geste",
        "Set the MYIRO-1 flat on its cap, then click “Start calibration”.",
    ),
    ("ecran.etalonnage.lancer", "Start calibration"),
    ("ecran.etalonnage.annuler", "Cancel"),
    (
        "ecran.etalonnage.en_cours",
        "Calibrating. Leave the instrument on its cap for a few seconds.",
    ),
    ("ecran.etalonnage.reussi", "The MYIRO-1 is calibrated."),
    (
        "ecran.etalonnage.reussi.suite",
        "You can now measure your colours.",
    ),
    ("ecran.etalonnage.continuer", "Continue"),
    (
        "ecran.etalonnage.schema",
        "Diagram: the MYIRO-1 set flat on its cap, which holds the reference white.",
    ),
    ("ecran.etalonnage.schema.instrument", "MYIRO-1"),
    ("ecran.etalonnage.schema.capuchon", "Cap"),
    ("ecran.etalonnage.schema.blanc", "Reference white"),
    // Spot measurement: Measure sheet ({n} is replaced)
    ("mesurer.nom_defaut", "Colour {n}"),
    ("mesurer.declenchement", "Measurement"),
    ("mesurer.declenchement.automatique", "Automatic"),
    ("mesurer.declenchement.manuel", "Manual"),
    (
        "mesurer.declenchement.non_retenu",
        "This choice applies to this session: it could not be saved for next time.",
    ),
    (
        "mesurer.consigne.automatique",
        "Set the MYIRO-1 on the colour, then click “Measure”.",
    ),
    (
        "mesurer.consigne.manuel",
        "Set the MYIRO-1 flat on the colour, click “Measure”, then press the instrument’s button.",
    ),
    ("mesurer.action", "Measure"),
    (
        "mesurer.en_cours.automatique",
        "Measuring. Keep the MYIRO-1 still on the colour.",
    ),
    (
        "mesurer.en_cours.manuel",
        "Press the MYIRO-1 button and keep it still until the measurement is over.",
    ),
    ("mesurer.liste", "Latest measurements"),
    ("mesurer.non_rangee", "Not stored"),
    (
        "mesurer.raison.aucune_condition",
        "Available once a printing condition exists. Add one on the left.",
    ),
    (
        "mesurer.raison.repos",
        "Available once the instrument is reconnected.",
    ),
    ("mesurer.valeurs", "Values"),
    (
        "mesurer.calcul",
        "Computed by myiro-libre from the spectrum, with its own tables: illuminant D50, 2° observer.",
    ),
    ("mesurer.ranger", "Store again"),
    ("mesurer.couleur", "Colour on screen"),
    ("mesurer.approchee", "Approximate colour on screen"),
    (
        "mesurer.erreur.sans_instrument.cause",
        "No instrument is ready to measure.",
    ),
    (
        "mesurer.erreur.sans_instrument.action",
        "Plug in the MYIRO-1 and wait for it to appear at the top of the window.",
    ),
    (
        "mesurer.erreur.condition_absente.cause",
        "This printing condition is no longer in the library. Nothing was measured.",
    ),
    (
        "mesurer.erreur.condition_absente.action",
        "Choose another one in the list, then measure again.",
    ),
    (
        "mesurer.erreur.bibliotheque_fermee.cause",
        "The library could not open. Nothing was measured.",
    ),
    (
        "mesurer.erreur.bibliotheque_fermee.action",
        "Close and restart myiro-libre.",
    ),
    ("mesurer.lab", "L*, a*, b*"),
    ("mesurer.lch", "L*, C*, h°"),
    ("mesurer.xyz", "X, Y, Z"),
    // Reference colour and difference (ticket #8)
    ("mesurer.reference.titre", "Reference colour"),
    (
        "mesurer.reference.aucune_texte",
        "No reference colour. Use a measurement as reference to see how far the others are from it.",
    ),
    ("mesurer.reference.designer", "Use as reference"),
    ("mesurer.reference.retirer", "Remove reference"),
    ("mesurer.reference.marque", "Reference"),
    ("mesurer.reference.actuelle", "Current reference colour: “{nom}”."),
    ("mesurer.conditions.a_confirmer", "Measurement conditions to be confirmed."),
    ("mesurer.conditions.inconnues", "Measurement conditions unknown."),
    (
        "mesurer.reference.est",
        "This measurement is the reference colour. The other measurements show their difference from it.",
    ),
    ("mesurer.reference.seuil", "Accepted difference (ΔE00)"),
    (
        "mesurer.reference.seuil_aide",
        "Leave empty not to judge the difference. It is kept with the reference colour.",
    ),
    ("mesurer.reference.seuil_non_fixe", "No limit set"),
    (
        "mesurer.reference.seuil_non_fixe_texte",
        "No limit set: no verdict. Set the accepted difference on the reference colour.",
    ),
    (
        "mesurer.reference.non_rangee",
        "This measurement is not stored in the library, where the reference is kept. First click “Store again”.",
    ),
    (
        "mesurer.reference.aucune",
        "First choose a reference colour.",
    ),
    (
        "mesurer.reference.seuil_invalide",
        "The accepted difference must be a number greater than zero, for example 2.5.",
    ),
    ("mesurer.ecart.titre", "Difference from reference"),
    ("mesurer.ecart.de00", "ΔE00 (CIEDE2000)"),
    ("mesurer.ecart.dc", "ΔC (saturation)"),
    ("mesurer.ecart.dh", "ΔH (hue)"),
    ("mesurer.ecart.accepte", "Accepted difference"),
    ("mesurer.ecart.comparaison", "Top: reference colour, bottom: this measurement"),
    ("mesurer.liste.ecart", "ΔE00 (CIEDE2000)"),
    ("mesurer.liste.verdict", "Verdict"),
    ("verdict.conforme", "Within tolerance"),
    ("verdict.proche_de_la_limite", "Close to the limit"),
    ("verdict.hors_tolerance", "Out of tolerance"),
    ("verdict.inconnu", "Unknown difference"),
    ("verdict.non_comparable", "Different conditions"),
    ("mesurer.verdict.conforme", "The difference stays within the limit set."),
    (
        "mesurer.verdict.proche_de_la_limite",
        "The difference stays within the limit set, but is getting close to it.",
    ),
    ("mesurer.verdict.hors_tolerance", "The difference exceeds the limit set."),
    (
        "mesurer.verdict.inconnu",
        "The difference cannot be computed: the values of one of the two measurements are unknown.",
    ),
    (
        "mesurer.ecart.condition_differente",
        "This measurement and the reference colour do not share the same measurement condition: they cannot be compared, no difference and no verdict.",
    ),
    (
        "mesurer.ecart.non_confirmee",
        "The measurement condition is not confirmed for both measurements: treat this difference with caution.",
    ),
    ("mesurer.ecart.calcul_titre", "How the difference is computed"),
    (
        "mesurer.ecart.calcul",
        "ΔE00: CIEDE2000 formula, factors k_L = k_C = k_H = 1.\nΔC and ΔH: CIELAB chroma and hue differences, signed (positive: measurement more saturated, hue turned counter-clockwise).\nLab D50, 2° observer, computed by myiro-libre from spectra of the same measurement condition.\nClose to the limit: from 80% of the accepted difference, a choice made by myiro-libre, not by a standard. The verdict applies to the difference as written, to two decimals.\nDifferent measurement conditions: no difference and no verdict.",
    ),
    ("cartouche.micrologiciel", "Firmware"),
    ("cartouche.date", "Date"),
    (
        "mesurer.erreur.rangement",
        "The library could not store this measurement. It stays shown here until myiro-libre is closed.",
    ),
    // Spot measurement: problems
    (
        "probleme.mesure_echouee.cause",
        "The measurement failed. The MYIRO-1 may not have been flat on the colour.",
    ),
    (
        "probleme.mesure_echouee.action",
        "Set it flat, click “Measure”, then press its button.",
    ),
    (
        "probleme.mesure_delai.cause",
        "The MYIRO-1 button was not pressed in time.",
    ),
    (
        "probleme.mesure_delai.action",
        "Click “Measure” again, then press the instrument’s button within two minutes.",
    ),
    (
        "probleme.mesure_delai_automatique.cause",
        "The MYIRO-1 did not finish the measurement in time.",
    ),
    (
        "probleme.mesure_delai_automatique.action",
        "Check that it sits flat on the colour, then click “Measure” again.",
    ),
    (
        "probleme.declenchement_refuse.cause",
        "The MYIRO-1 refused to start the measurement without its button being pressed.",
    ),
    (
        "probleme.declenchement_refuse.action",
        "Choose “Measurement: manual”, click “Measure”, then press the instrument’s button.",
    ),
    (
        "probleme.etalonnage_a_refaire.cause",
        "The MYIRO-1 needs to be calibrated again.",
    ),
    (
        "probleme.etalonnage_a_refaire.action",
        "Click “Calibrate” at the top, then measure again.",
    ),
    (
        "probleme.repos_incertain.cause",
        "The MYIRO-1 did not confirm the end of the measurement. The measurement is kept.",
    ),
    (
        "probleme.repos_incertain.action",
        "To measure again, click “Try again”: the instrument will be reconnected, then needs calibrating.",
    ),
    // Profile verification (ticket #41)
    (
        "profil.erreur.outil_manquant",
        "A tool of ArgyllCMS is missing from the argyll folder next to the application. Replace this folder with a complete copy of ArgyllCMS 3.5.0, then restart the application.",
    ),
    ("verif.titre", "Verify the profile"),
    ("verif.profil", "Profile to verify"),
    ("verif.mesure", "Measurement of the second chart"),
    ("verif.condition", "Measurement condition"),
    ("verif.tolerances", "Tolerances"),
    ("verif.fiche", "Print sheet"),
    ("verif.fiche.impression", "How the second chart was printed"),
    (
        "verif.fiche.impression.a_travers_le_profil",
        "Through this profile, from sRGB colours, relative colorimetric",
    ),
    (
        "verif.fiche.impression.sans_gestion",
        "Without colour management",
    ),
    ("verif.fiche.logiciel", "Printing software"),
    ("verif.fiche.support_pilote", "Media chosen in the driver"),
    ("verif.fiche.qualite", "Quality and resolution"),
    ("verif.fiche.encres", "Inks"),
    ("verif.fiche.remarques", "Notes"),
    ("verif.fiche.confirmer", "I confirm these settings"),
    ("verif.lancer", "Verify"),
    ("verif.en_cours", "Verifying…"),
    ("verif.avance", "Advanced"),
    ("verif.fidelite", "Fidelity to the requested colours"),
    ("verif.prediction", "Agreement with the profile’s prediction"),
    ("verif.hors_gamut", "Out-of-gamut colours"),
    (
        "verif.inconnu.sans_demande",
        "unknown: the chart was printed without colour management, no colour was requested",
    ),
    (
        "verif.inconnu.aucune_plage_dans_le_gamut",
        "unknown: no requested colour is inside the profile’s gamut",
    ),
    ("verif.inconnu.aucune_plage", "no patch"),
    ("verif.inconnu", "unknown"),
    ("verif.etat.non_verifie", "To be verified"),
    ("verif.etat.accepte", "Accepted under {tolerances}"),
    ("verif.etat.hors_tolerance", "Out of tolerance"),
    ("verif.etat.incomplet", "Incomplete verification"),
    ("verif.critere.moyenne", "Mean ΔE00"),
    ("verif.critere.p95", "95% of patches"),
    ("verif.critere.max", "Maximum ΔE00"),
    ("verif.plages", "Patches"),
    ("verif.pires", "Furthest patches"),
    ("verif.respecte", "met"),
    ("verif.depasse", "exceeded"),
    ("verif.calculee", "computed by ArgyllCMS"),
    ("verif.demande", "Requested colour"),
    ("verif.appareil", "Value after transform"),
    ("verif.predite", "Profile prediction"),
    ("verif.mesuree", "Measurement"),
    (
        "verif.memes_couleurs",
        "Same colours as the profile’s chart: a less demanding check than a different chart.",
    ),
    (
        "verif.regle",
        "An exceeded threshold gives “out of tolerance”. Otherwise, a criterion that cannot be computed gives “incomplete verification”. Otherwise, the profile is accepted.",
    ),
    ("verif.recalculer", "Recompute with other tolerances"),
    ("verif.exporter_rapport", "Export the report"),
    (
        "verif.exporter_profil",
        "Export the profile with its report",
    ),
    ("verif.historique", "Verifications of this profile"),
    ("verif.derniere", "Latest verification"),
    (
        "verif.indisponible.sans_profil",
        "No active RGB profile to verify in this resolution.",
    ),
    (
        "verif.refus.fiche",
        "The print sheet is not confirmed. Check the settings, then tick the confirmation box.",
    ),
    (
        "verif.refus.mesure_du_profil",
        "This measurement was used to compute the profile: it cannot verify it. Choose the measurement of a second chart.",
    ),
    (
        "verif.refus.cmjn",
        "Verifying a CMYK profile is not available yet. Choose an RGB profile.",
    ),
    (
        "verif.refus.vide",
        "This measurement contains no patch. Import another file.",
    ),
    (
        "verif.refus.tailles",
        "ArgyllCMS did not return a value for every patch. Nothing was saved. Run the verification again.",
    ),
    (
        "verif.refus.valeur",
        "A patch of this measurement or a profile value is unusable. Nothing was saved. Check the measurement, then import it again.",
    ),
    (
        "verif.erreur.calcul",
        "ArgyllCMS could not query the profile. Nothing was saved. Check the profile, then run the verification again.",
    ),
    ("verif.tolerances.nom", "Set name"),
    ("verif.tolerances.seuil_gamut", "Out-of-gamut threshold (ΔE00)"),
    ("verif.tolerances.enregistrer", "Save"),
    (
        "verif.tolerances.proposition",
        "Suggestion, to be confirmed: no value is imposed.",
    ),
    (
        "verif.tolerances.enregistre",
        "{jeu} saved; verifications already made do not change.",
    ),
    (
        "verif.tolerances.aucun_seuil",
        "This set has no threshold. Fill in at least one threshold, then save.",
    ),
    (
        "verif.tolerances.seuil_invalide",
        "A threshold is not a number greater than zero. Correct it, then save.",
    ),
    (
        "verif.tolerances.aucun",
        "No tolerance set saved. Save one before verifying.",
    ),
    // Verification report (HTML)
    ("rapport.titre", "Profile verification report"),
    ("rapport.sans_seuil", "no threshold"),
    ("rapport.resultats", "Results"),
    ("rapport.controles", "Checks one by one"),
    ("rapport.famille", "Family"),
    ("rapport.critere", "Criterion"),
    ("rapport.limite", "Threshold"),
    ("rapport.valeur", "Value found"),
    ("rapport.resultat", "Result"),
    ("rapport.toutes_plages", "All patches"),
    ("rapport.plage", "Patch"),
    ("rapport.rvb", "Chart RGB (0 to 255)"),
    ("rapport.ecart_fidelite", "Fidelity ΔE00"),
    ("rapport.ecart_prediction", "Prediction ΔE00"),
    ("rapport.ecart_gamut", "ΔE00 to gamut"),
    ("rapport.recalcul", "Recomputed without a new measurement from the verification of"),
    ("rapport.date", "Date"),
    ("rapport.profil", "Verified profile"),
    ("rapport.nom", "Name"),
    ("rapport.empreinte", "SHA-256 fingerprint"),
    ("rapport.recette", "Calculation recipe"),
    ("rapport.argyll", "ArgyllCMS version"),
    ("rapport.source", "Measurement source"),
    ("rapport.fichier", "Imported file"),
    ("rapport.instrument", "Instrument declared by the file"),
    ("rapport.supposee", "assumed from the manufacturer’s convention"),
    ("rapport.commandes", "ArgyllCMS commands"),
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
        for etat in ["detecte", "connecte", "etalonnage_requis", "etalonne"] {
            demandees.push(format!("instrument.etat.{etat}"));
        }
        demandees.extend(
            [
                "instrument.recherche",
                "instrument.aucun",
                "instrument.barre",
                "instrument.logiciel_absent",
                "raison.etalonnage",
                "ecran.choix.titre.logiciel_absent",
                "ecran.choix.titre.logiciel_inutilisable",
                "choix.ligne",
                "choix.liaison.usb",
                "choix.liaison.reseau",
                "choix.en_service",
                "choix.utiliser",
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
        for bouton in [
            "ecran.reessayer",
            "ecran.autoriser_pare_feu",
            "ecran.choix.action",
            "ecran.etalonnage.lancer",
        ] {
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

    /// Chaque texte demandé par la feuille Mesurer (`t("…")` dans
    /// `mesurer.js`) existe au catalogue.
    #[test]
    fn la_feuille_mesurer_ne_demande_que_des_cles_du_catalogue() {
        let script = include_str!("../interface/mesurer.js");
        // `t("…")` seul, pas la fin d'un autre nom (`CustomEvent("…")`).
        let demandees: Vec<&str> = script
            .match_indices("t(\"")
            .filter(|(i, _)| {
                !script[..*i]
                    .chars()
                    .next_back()
                    .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.')
            })
            .filter_map(|(i, _)| script[i + 3..].split('"').next())
            .collect();
        assert!(demandees.len() > 10, "clés lues : {demandees:?}");
        let inconnues: Vec<_> = demandees
            .iter()
            .filter(|c| !cles().any(|k| k == **c))
            .collect();
        assert!(
            inconnues.is_empty(),
            "clés absentes du catalogue : {inconnues:?}"
        );
    }

    /// L'illuminant et l'observateur affichés sont ceux de notre calcul, pas
    /// une affirmation sur l'instrument : le texte le dit.
    #[test]
    fn l_observateur_affiche_est_celui_de_notre_calcul() {
        for langue in [Langue::Francais, Langue::Anglais] {
            let calcul = texte(langue, "mesurer.calcul");
            assert!(calcul.contains("myiro-libre"), "{calcul}");
            assert!(calcul.contains("D50") && calcul.contains("2°"), "{calcul}");
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
