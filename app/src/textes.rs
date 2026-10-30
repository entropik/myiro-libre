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
    // Titre de la fenêtre, avec `--bibliotheque <dossier>`
    ("fenetre.titre.essai", "myiro-libre — bibliothèque d’essai"),
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
    (
        "profil.erreur.outil_manquant",
        "Un outil d’ArgyllCMS manque dans le dossier argyll à côté de l’application. Remplacez ce dossier par une copie complète d’ArgyllCMS 3.5.0, puis relancez l’application.",
    ),
    (
        "comparer.refus.en_tete",
        "Ce fichier n’est pas un profil ICC lisible.",
    ),
    (
        "comparer.refus.pas_rvb",
        "Ce profil n’est pas un profil RVB. La comparaison CMJN n’est pas encore prévue.",
    ),
    (
        "comparer.refus.sans_table",
        "Ce profil n’a pas de table colorimétrique (A2B1)\u{202f}: c’est peut-être un profil d’écran. Il n’est pas comparé.",
    ),
    (
        "comparer.refus.structure",
        "La table colorimétrique de ce profil est d’une structure que myiro-libre ne lit pas encore ({detail}). Il n’est pas comparé.",
    ),
    (
        "comparer.refus.tronque",
        "Ce profil est incomplet\u{202f}: une partie ({detail}) sort du fichier. Il n’est pas comparé.",
    ),
    (
        "comparer.refus.sans_blanc",
        "Ce profil ne dit pas quel est le blanc du support\u{202f}: ses couleurs absolues ne se calculent pas. Il n’est pas comparé.",
    ),
    (
        "comparer.refus.sans_table_inverse",
        "Ce profil n’a pas de table inverse lisible (B2A1)\u{202f}: on ne sait pas quelles couleurs l’imprimante rendrait de l’image. Ce point reste inconnu.",
    ),
    (
        "comparer.refus.trop_de_profils",
        "On compare quatre profils au plus. Retirez-en un, puis recommencez.",
    ),
    (
        "comparer.refus.fichier_ferme",
        "Ce fichier n’est plus ouvert. Ouvrez-le de nouveau.",
    ),
    (
        "comparer.refus.aucun_profil",
        "Choisissez au moins un profil à comparer.",
    ),
    (
        "comparer.refus.profil_inconnu",
        "Ce profil n’est plus dans la bibliothèque. Choisissez-en un autre dans la liste.",
    ),
    (
        "comparer.refus.fichier_illisible",
        "Ce fichier ne peut pas être lu. Vérifiez qu’il existe toujours, puis ouvrez-le de nouveau.",
    ),
    (
        "comparer.refus.fichier_trop_grand",
        "Ce fichier est bien trop grand pour un profil ICC\u{202f}: il n’est pas ouvert.",
    ),
    (
        "comparer.image.refus.en_tete",
        "Le profil embarqué dans cette image n’est pas un profil ICC lisible ({detail}).",
    ),
    (
        "comparer.image.refus.pas_rvb",
        "Le profil embarqué dans cette image est un profil CMJN. Les images CMJN sont prévues dans une seconde étape.",
    ),
    (
        "comparer.image.refus.sans_conversion",
        "Le profil embarqué dans cette image ne dit pas quelles couleurs ses pixels demandent\u{202f}: il n’a ni table A2B, ni matrice et courbes, ni courbe de gris.",
    ),
    (
        "comparer.image.refus.structure",
        "Le profil embarqué dans cette image est d’une structure que myiro-libre ne lit pas encore ({detail}).",
    ),
    (
        "comparer.image.refus.tronque",
        "Le profil embarqué dans cette image est incomplet\u{202f}: une partie ({detail}) sort du profil.",
    ),
    (
        "comparer.image.refus.illisible",
        "Le profil embarqué dans cette image est abîmé ou incomplet\u{202f}: il ne se lit pas ({detail}).",
    ),
    (
        "image.ouvrir",
        "Ajouter une image",
    ),
    (
        "image.filtre",
        "Images JPEG, PNG ou TIFF",
    ),
    (
        "image.traiter_srvb",
        "Traiter comme sRVB",
    ),
    (
        "image.refus.cmjn",
        "Cette image est en CMJN. La comparaison des images CMJN viendra dans une seconde étape. Ouvrez une version RVB de la photo.",
    ),
    (
        "image.refus.profil",
        "Le profil intégré à l’image ne se lit pas ({detail}). Choisissez «\u{202f}Traiter comme sRVB\u{202f}» si la photo vient d’un appareil ou d’un site en sRVB.",
    ),
    (
        "image.refus.trop_grande",
        "Cette image est trop grande pour être ouverte (plus de 120 millions de pixels, ou un fichier de plus de 1\u{202f}Go). Ouvrez une version réduite de la photo.",
    ),
    (
        "image.refus.illisible",
        "Ce fichier ne se lit pas comme une image. Vérifiez qu’il existe toujours et qu’il s’ouvre dans un autre logiciel, puis ouvrez-le de nouveau.",
    ),
    (
        "image.refus.format",
        "Ce format d’image n’est pas pris en charge. Ouvrez la photo en JPEG, PNG ou TIFF.",
    ),
    (
        "image.refus.aucune",
        "Aucune image n’est ouverte. Ajoutez-en une, puis recommencez.",
    ),
    (
        "image.refus.sans_ecarts",
        "Les écarts de ce profil ne sont pas calculés. Relancez l’analyse de l’image.",
    ),
    (
        "image.annulee",
        "Analyse de l’image annulée.",
    ),
    (
        "image.erreur.calcul",
        "Le calcul de ce profil s’est arrêté sur une erreur de myiro-libre. Ce point reste inconnu.",
    ),
    (
        "image.profil.embarque",
        "Profil de l’image\u{202f}: {nom}",
    ),
    (
        "image.profil.sans_nom",
        "profil intégré sans nom",
    ),
    (
        "image.profil.ecartee",
        "Table {table} du profil de l’image non utilisée\u{202f}: {raison}",
    ),
    (
        "image.profil.srvb",
        "Profil de l’image\u{202f}: sRVB (aucun profil intégré)",
    ),
    (
        "image.profil.suppose",
        "sRVB, supposé\u{202f}: choisi par vous",
    ),
    (
        "image.approche",
        "Calculé sur 1 pixel sur {n}",
    ),
    (
        "image.seize_bits",
        "Valeurs 16 bits ramenées à 8 bits",
    ),
    (
        "image.titre",
        "Image",
    ),
    (
        "image.aide",
        "Ouvrez une photo de client en RVB (JPEG, PNG ou TIFF)\u{202f}: pour chaque profil affiché, la page montre les pixels que l’imprimante ne pourra pas reproduire. L’image reste en mémoire le temps de la comparaison\u{202f}; elle n’est ni rangée ni copiée.",
    ),
    (
        "image.remplacer",
        "Ouvrir une autre image",
    ),
    (
        "image.retirer",
        "Retirer l’image",
    ),
    (
        "image.en_cours",
        "Analyse de l’image en cours…",
    ),
    (
        "image.annuler",
        "Annuler",
    ),
    (
        "image.relancer",
        "Relancer l’analyse",
    ),
    (
        "image.taille",
        "Image d’analyse de {l} × {h} pixels, tirée d’une photo de {lo} × {ho} pixels",
    ),
    (
        "image.surimpression.profil",
        "Profil de la surimpression",
    ),
    (
        "image.surimpression.inconnue",
        "Pas de surimpression pour ce profil\u{202f}: {raison}",
    ),
    (
        "image.seuil.valeur",
        "Seuil d’alerte {s}\u{202f}ΔE00",
    ),
    (
        "image.affichage",
        "Afficher",
    ),
    (
        "image.affichage.image",
        "Image",
    ),
    (
        "image.affichage.les_deux",
        "Image et alerte",
    ),
    (
        "image.affichage.alerte",
        "Alerte seule",
    ),
    (
        "nuage.bascule",
        "Couleurs de l’image",
    ),
    (
        "nuage.aide",
        "Chaque point est la moyenne d’un groupe de couleurs proches de l’image, placée sur le blanc du papier de {profil}. Il passe dans la couleur d’alerte dès qu’une de ses couleurs dépasse {s}\u{202f}ΔE00.",
    ),
    (
        "nuage.indisponible",
        "Les couleurs de l’image apparaissent ici quand une image est analysée face au profil choisi pour la surimpression.",
    ),
    (
        "nuage.aria",
        "Couleurs de l’image\u{202f}: {n} points, dont {a} au-delà du seuil d’alerte.",
    ),
    (
        "nuage.compte",
        "Couleurs de l’image dans la coupe (L* {l} ± {d})\u{202f}: {n} points, dont {a} au-delà du seuil d’alerte.",
    ),
    (
        "nuage.legende",
        "Couleurs de l’image, L* {l} ± {d}",
    ),
    (
        "nuage.legende.alerte",
        "Couleur de l’image au-delà du seuil d’alerte",
    ),
    (
        "image.legende",
        "Écart d’au plus {s}\u{202f}ΔE00, {profil}, sans compensation du point noir. En vert, au-delà.",
    ),
    (
        "image.canvas.aria",
        "{p}\u{202f}% de l’image au-delà de {s}\u{202f}ΔE00 avec {profil}. Cliquez sur l’image pour voir la couleur d’un point et sa couleur imprimée.",
    ),
    (
        "image.chiffres",
        "Chiffres de l’image",
    ),
    (
        "image.chiffres.part",
        "Part de l’image au-delà de {s}\u{202f}ΔE00",
    ),
    (
        "image.classe.leger",
        "Léger, de 2 à 5\u{202f}ΔE00",
    ),
    (
        "image.classe.moyen",
        "Moyen, de 5 à 10\u{202f}ΔE00",
    ),
    (
        "image.classe.fort",
        "Fort, plus de 10\u{202f}ΔE00",
    ),
    (
        "image.chiffres.teintes",
        "Teintes les plus touchées",
    ),
    (
        "image.chiffres.inconnus",
        "Pixels à l’écart inconnu",
    ),
    (
        "image.chiffres.couleurs",
        "Couleurs distinctes",
    ),
    (
        "image.chiffres.raison",
        "Raison",
    ),
    (
        "image.valeur.pourcent",
        "{p}\u{202f}%",
    ),
    (
        "image.teinte.valeur",
        "{nom} {p}\u{202f}%",
    ),
    (
        "image.teintes.aucune",
        "aucune",
    ),
    (
        "image.inconnus.valeur",
        "{n} pixels ({p}\u{202f}%), jamais comptés comme bons",
    ),
    (
        "image.inconnus.aucun",
        "aucun",
    ),
    (
        "image.teinte.rouge",
        "Rouges",
    ),
    (
        "image.teinte.jaune",
        "Jaunes",
    ),
    (
        "image.teinte.vert",
        "Verts",
    ),
    (
        "image.teinte.cyan",
        "Cyans",
    ),
    (
        "image.teinte.bleu",
        "Bleus",
    ),
    (
        "image.teinte.magenta",
        "Magentas",
    ),
    (
        "image.teinte.gris",
        "Gris (C* < 5)",
    ),
    (
        "image.verdict",
        "{p}\u{202f}% de l’image est imprimable sur ce papier avec ce profil",
    ),
    (
        "image.verdict.plus_de",
        "plus de {p}",
    ),
    (
        "image.verdict.moins_de",
        "moins de {p}",
    ),
    (
        "image.verdict.inconnus",
        "\u{202f}; {p}\u{202f}% inconnus",
    ),
    (
        "image.verdict.inconnu",
        "Part imprimable inconnue avec ce profil",
    ),
    (
        "image.point_noir",
        "Compensation du point noir",
    ),
    (
        "image.point_noir.aide",
        "Le noir de l’image est ramené sur le noir le plus profond du papier, comme le fait le pilote d’impression en colorimétrique relatif avec compensation. Changer ce réglage relance l’analyse.",
    ),
    (
        "image.legende.point_noir",
        "Écart d’au plus {s}\u{202f}ΔE00, {profil}, avec compensation du point noir. En vert, au-delà.",
    ),
    (
        "image.rendu.point_noir",
        "Colorimétrique relatif, avec compensation du point noir",
    ),
    (
        "image.rendu.relatif",
        "Colorimétrique relatif, sans compensation du point noir",
    ),
    (
        "image.agrandir",
        "Agrandir",
    ),
    (
        "image.agrandie.titre",
        "Image agrandie",
    ),
    (
        "image.agrandie.fermer",
        "Fermer",
    ),
    (
        "image.agrandie.aide",
        "Cliquez sur l’image pour voir la couleur d’un point. Échap ou un clic à côté de l’image pour revenir.",
    ),
    (
        "pastilles.pipette.visee",
        "Couleur visée après compensation du point noir\u{202f}: {lab}, à {e}\u{202f}ΔE00 de la couleur de l’image",
    ),
    (
        "pastilles.pipette.visee_identique",
        "Compensation du point noir\u{202f}: la couleur visée est celle de l’image.",
    ),
    (
        "pastilles.aria.visee",
        "Couleur visée\u{202f}: L* {l}, a* {a}, b* {b}",
    ),
    (
        "image.pixel.hors_image",
        "Ce point est en dehors de l’image.",
    ),
    (
        "image.pixel.transparent",
        "Ce point de l’image est transparent\u{202f}: il n’a pas de couleur à imprimer.",
    ),
    (
        "image.pixel.inconnu",
        "L’écart de cette couleur n’a pas pu être calculé.",
    ),
    (
        "pastilles.titre",
        "Couleurs hors gamut",
    ),
    (
        "pastilles.lecture",
        "À gauche la couleur de l’image, à droite la couleur imprimée avec le profil choisi\u{202f}; dessous, l’écart et la part de l’image. Les plus présentes d’abord.",
    ),
    (
        "pastilles.aucune",
        "Aucune des couleurs les plus présentes de l’image n’est au-delà de {s}\u{202f}ΔE00 avec {profil}.",
    ),
    (
        "pastilles.precaution",
        "Couleurs approchées à l’écran (écran non calibré).",
    ),
    (
        "pastilles.ecart",
        "{e}\u{202f}ΔE00",
    ),
    (
        "pastilles.part",
        "{p}\u{202f}%",
    ),
    (
        "pastilles.aria.image",
        "Couleur de l’image\u{202f}: L* {l}, a* {a}, b* {b}",
    ),
    (
        "pastilles.aria.imprimee",
        "Couleur imprimée\u{202f}: L* {l}, a* {a}, b* {b}",
    ),
    (
        "pastilles.image",
        "Image",
    ),
    (
        "pastilles.imprimee",
        "Imprimé",
    ),
    (
        "pastilles.lab",
        "L* {l}  a* {a}  b* {b}",
    ),
    (
        "pastilles.pipette.titre",
        "Couleur du point cliqué",
    ),
    (
        "pastilles.pipette.aide",
        "Cliquez sur l’image pour voir la couleur d’un point et sa couleur imprimée.",
    ),
    (
        "pastilles.pipette.au_dela",
        "Au-delà du seuil d’alerte de {s}\u{202f}ΔE00",
    ),
    (
        "pastilles.pipette.en_deca",
        "En deçà du seuil d’alerte de {s}\u{202f}ΔE00",
    ),
    (
        "pastilles.pipette.inconnu",
        "Couleur imprimée inconnue avec {profil}\u{202f}: {raison}",
    ),
    (
        "comparer.zone.gris",
        "Gris",
    ),
    (
        "comparer.zone.ombres",
        "Ombres",
    ),
    (
        "comparer.zone.clairs",
        "Tons clairs",
    ),
    (
        "comparer.zone.saturee.rouge",
        "Rouges saturés",
    ),
    (
        "comparer.zone.saturee.jaune",
        "Jaunes saturés",
    ),
    (
        "comparer.zone.saturee.vert",
        "Verts saturés",
    ),
    (
        "comparer.zone.saturee.cyan",
        "Cyans saturés",
    ),
    (
        "comparer.zone.saturee.bleu",
        "Bleus saturés",
    ),
    (
        "comparer.zone.saturee.magenta",
        "Magentas saturés",
    ),
    (
        "comparer.zone.moyenne.rouge",
        "Rouges moyens",
    ),
    (
        "comparer.zone.moyenne.jaune",
        "Jaunes moyens",
    ),
    (
        "comparer.zone.moyenne.vert",
        "Verts moyens",
    ),
    (
        "comparer.zone.moyenne.cyan",
        "Cyans moyens",
    ),
    (
        "comparer.zone.moyenne.bleu",
        "Bleus moyens",
    ),
    (
        "comparer.zone.moyenne.magenta",
        "Magentas moyens",
    ),
    (
        "comparer.fidelite.sans_mesures",
        "Inconnue\u{202f}: ce profil n’embarque pas ses mesures.",
    ),
    (
        "comparer.fidelite.illisible",
        "Inconnue\u{202f}: les mesures embarquées dans ce profil ne se lisent pas.",
    ),
    (
        "comparer.fidelite.echelle_spectre",
        "Inconnue\u{202f}: la plus forte valeur des spectres embarqués est entre 1,5 et 2. On ne sait pas s’ils sont en réflectance de 0 à 1 ou en pourcentage, et rien n’est deviné.",
    ),
    (
        "comparer.fidelite.rvb_inconnus",
        "Inconnue\u{202f}: les mesures embarquées ne disent pas quelles valeurs RVB ont été imprimées, ou dans quelle échelle.",
    ),
    (
        "comparer.fidelite.rvb_hors_echelle",
        "Inconnue\u{202f}: une plage des mesures embarquées a une valeur RVB hors de l’échelle prévue.",
    ),
    (
        "comparer.fidelite.lab_inconnu",
        "Inconnue\u{202f}: une plage des mesures embarquées n’a ni Lab ni spectre utilisable.",
    ),
    (
        "comparer.limite.sans_argyll",
        "La limite du gamut demande ArgyllCMS 3.5.0, introuvable à côté de l’application. La surface brute est montrée, et le reste de la comparaison est calculé.",
    ),
    (
        "comparer.limite.sans_iccgamut",
        "L’outil iccgamut d’ArgyllCMS manque\u{202f}: la limite du gamut n’est pas calculée, le reste de la comparaison l’est. Remplacez le dossier argyll par une copie complète d’ArgyllCMS 3.5.0.",
    ),
    (
        "comparer.limite.argyll_version",
        "ArgyllCMS {version} a été trouvé, mais seule la version 3.5.0 est validée\u{202f}: la limite du gamut n’est pas calculée, le reste de la comparaison l’est.",
    ),
    (
        "comparer.limite.argyll_illisible",
        "ArgyllCMS ne répond pas comme prévu\u{202f}: la limite du gamut n’est pas calculée, le reste de la comparaison l’est.",
    ),
    (
        "comparer.limite.fichier",
        "Un fichier de travail d’iccgamut n’a pas pu être lu ou écrit\u{202f}: la limite du gamut n’est pas calculée, le reste de la comparaison l’est. Réessayez.",
    ),
    (
        "comparer.limite.illisible",
        "La limite du gamut écrite par iccgamut ne se lit pas\u{202f}: elle n’est pas montrée, le reste de la comparaison est calculé.",
    ),
    (
        "comparer.limite.interrompue",
        "iccgamut s’est arrêté avant la fin, sans dire d’erreur\u{202f}: la limite du gamut n’est pas calculée, le reste de la comparaison l’est. Recommencez la comparaison.",
    ),
    (
        "comparer.erreur.limite",
        "Le calcul de la limite du gamut a échoué dans ArgyllCMS (iccgamut)\u{202f}: elle n’est pas montrée, le reste de la comparaison est calculé.",
    ),
    (
        "comparer.origine.bibliotheque",
        "Bibliothèque\u{202f}: {chemin}, profil du {date}",
    ),
    (
        "comparer.origine.fichier",
        "Fichier\u{202f}: {nom} (ouvert pour la comparaison, pas dans la bibliothèque)",
    ),
    (
        "comparer.inconnu",
        "inconnu",
    ),
    (
        "comparer.vue3d.aria",
        "Gamuts en volume (flèches pour tourner, + et − pour zoomer, 0 pour revenir à la vue de départ)\u{202f}: {profils}.",
    ),
    (
        "comparer.vue3d.aria.profil",
        "{nom}, volume {volume} unités L*a*b* au cube, blanc du papier L* {blanc}",
    ),
    (
        "comparer.vue3d.aria.vide",
        "Gamuts en volume\u{202f}: aucun profil affiché.",
    ),
    (
        "comparer.vue3d.bulle",
        "{nom}\u{202f}: L* {l}, a* {a}, b* {b}",
    ),
    (
        "comparer.vue3d.blanc",
        "Blanc du papier L* {l}",
    ),
    (
        "comparer.graphique.teinte",
        "Teinte h (°)",
    ),
    (
        "comparer.graphique.chroma",
        "Chroma C*",
    ),
    (
        "comparer.graphique.clarte",
        "Clarté L*",
    ),
    (
        "comparer.graphique.position",
        "Position sur le chemin (0 à 255)",
    ),
    (
        "comparer.graphique.mesures",
        "Plages mesurées sur ce chemin",
    ),
    (
        "comparer.graphique.valeurs",
        "Valeurs",
    ),
    (
        "comparer.graphique.plage_mesuree",
        "{position} (plage mesurée)",
    ),
    (
        "comparer.graphique.teintes.aria",
        "Chroma maximale C* par teinte, de 0 à 360°\u{202f}: {profils}. Les valeurs se lisent dans le tableau «\u{202f}Valeurs\u{202f}».",
    ),
    (
        "comparer.graphique.coupe.aria",
        "Coupe du gamut à clarté L* {l}, dans le plan a*b*\u{202f}: {profils}. Les valeurs se lisent dans le tableau «\u{202f}Valeurs\u{202f}».",
    ),
    (
        "comparer.graphique.chemin.l.aria",
        "Clarté L* le long du chemin du cube RVB\u{202f}: {profils}. Les valeurs se lisent dans le tableau «\u{202f}Valeurs\u{202f}».",
    ),
    (
        "comparer.graphique.chemin.c.aria",
        "Chroma C* le long du chemin du cube RVB\u{202f}: {profils}. Les valeurs se lisent dans le tableau «\u{202f}Valeurs\u{202f}».",
    ),
    (
        "comparer.titre",
        "Comparer des profils",
    ),
    (
        "comparer.aide",
        "De un à quatre profils RVB, rangés dans la bibliothèque ou ouverts depuis un fichier. Le premier sert de référence aux écarts.",
    ),
    (
        "comparer.profils",
        "Profils comparés",
    ),
    (
        "comparer.ajouter",
        "Ajouter un profil",
    ),
    (
        "comparer.ajouter.chargement",
        "Lecture de la liste des profils en cours…",
    ),
    (
        "comparer.ajouter.max",
        "Quatre profils au plus\u{202f}: retirez-en un pour en ajouter un autre.",
    ),
    (
        "comparer.fermer",
        "Fermer la comparaison",
    ),
    (
        "comparer.en_cours",
        "Comparaison en cours\u{202f}: la limite du gamut peut prendre quelques secondes par profil.",
    ),
    (
        "comparer.ouvrir",
        "Comparer",
    ),
    (
        "comparer.ouvrir.actuel",
        "Comparer à l’actuel",
    ),
    (
        "comparer.retirer",
        "Retirer",
    ),
    (
        "comparer.reference",
        "référence",
    ),
    (
        "comparer.choix.titre",
        "Profils de la bibliothèque",
    ),
    (
        "comparer.choix.vide",
        "Aucun profil rangé dans la bibliothèque pour l’instant.",
    ),
    (
        "comparer.choix.actuel",
        "actuel",
    ),
    (
        "comparer.choix.fichier",
        "Ouvrir un fichier .icc",
    ),
    (
        "comparer.vide",
        "Ajoutez un profil pour commencer la comparaison.",
    ),
    (
        "comparer.chiffres",
        "Chiffres clés",
    ),
    (
        "comparer.chiffres.blanc",
        "Blanc du papier",
    ),
    (
        "comparer.chiffres.noir",
        "Noir",
    ),
    (
        "comparer.chiffres.noir.valeur",
        "L* {l} · densité ≈ {d}",
    ),
    (
        "comparer.chiffres.volume_surface",
        "Volume, surface brute",
    ),
    (
        "comparer.chiffres.volume_limite",
        "Volume, limite du gamut",
    ),
    (
        "comparer.chiffres.limite_indisponible",
        "non calculé",
    ),
    (
        "comparer.chiffres.fidelite",
        "Fidélité à ses mesures",
    ),
    (
        "comparer.chiffres.fidelite.valeur",
        "moyenne {moyenne}, p95 {p95}, max {max} ΔE00 sur {plages} plages",
    ),
    (
        "comparer.chiffres.repetabilite",
        "Répétabilité des mesures",
    ),
    (
        "comparer.chiffres.repetabilite.valeur",
        "moyenne {moyenne}, max {max} ΔE00 ({couleurs} couleurs, {paires} paires)",
    ),
    (
        "comparer.chiffres.repetabilite.inconnue",
        "inconnue\u{202f}: aucune plage mesurée deux fois",
    ),
    (
        "comparer.valeur.lab",
        "L* {l}, a* {a}, b* {b}",
    ),
    (
        "comparer.valeur.volume",
        "{volume} unités L*a*b* au cube",
    ),
    (
        "comparer.volume",
        "Gamut en volume",
    ),
    (
        "comparer.volume.affiches",
        "Profils affichés",
    ),
    (
        "comparer.volume.couleur",
        "Couleur",
    ),
    (
        "comparer.volume.couleur.serie",
        "Par profil",
    ),
    (
        "comparer.volume.couleur.reelle",
        "Couleurs réelles",
    ),
    (
        "comparer.volume.couleur.aide",
        "En couleurs réelles, seul le premier profil affiché est habillé de ses couleurs\u{202f}: le profil de référence, quand il est coché. Les autres profils affichés sont dessinés en fil de fer, dans leur couleur de profil, pour qu’on les distingue.",
    ),
    (
        "comparer.volume.forme",
        "Forme",
    ),
    (
        "comparer.volume.forme.limite",
        "Limite du gamut",
    ),
    (
        "comparer.volume.forme.surface",
        "Surface brute",
    ),
    (
        "comparer.volume.rendu",
        "Rendu",
    ),
    (
        "comparer.volume.rendu.opaque",
        "Opaque",
    ),
    (
        "comparer.volume.rendu.transparent",
        "Transparent",
    ),
    (
        "comparer.volume.aide",
        "Glisser pour tourner, molette pour zoomer\u{202f}; au clavier, flèches, + et −, 0 pour revenir.",
    ),
    (
        "comparer.volume.surface_montree",
        "{nom}\u{202f}: surface brute montrée. {raison}",
    ),
    (
        "comparer.teintes",
        "Chroma maximale par teinte",
    ),
    (
        "comparer.coupe",
        "Coupe à clarté constante",
    ),
    (
        "comparer.coupe.clarte",
        "Clarté L*",
    ),
    (
        "comparer.chemin",
        "Courbes le long d’un chemin du cube RVB",
    ),
    (
        "comparer.chemin.depart",
        "Départ",
    ),
    (
        "comparer.chemin.arrivee",
        "Arrivée",
    ),
    (
        "comparer.primaires",
        "Couleurs de base",
    ),
    (
        "comparer.primaire.blanc",
        "Blanc",
    ),
    (
        "comparer.primaire.noir",
        "Noir",
    ),
    (
        "comparer.primaire.rouge",
        "Rouge",
    ),
    (
        "comparer.primaire.vert",
        "Vert",
    ),
    (
        "comparer.primaire.bleu",
        "Bleu",
    ),
    (
        "comparer.primaire.cyan",
        "Cyan",
    ),
    (
        "comparer.primaire.magenta",
        "Magenta",
    ),
    (
        "comparer.primaire.jaune",
        "Jaune",
    ),
    (
        "comparer.primaires.couleur",
        "Couleur",
    ),
    (
        "comparer.primaires.profil",
        "Profil",
    ),
    (
        "comparer.primaires.pastille",
        "Pastille",
    ),
    (
        "comparer.primaires.approchee",
        "approchée",
    ),
    (
        "comparer.primaires.sans_teinte",
        "—",
    ),
    (
        "comparer.ecarts",
        "Écarts au profil de référence",
    ),
    (
        "comparer.ecarts.deux",
        "Les écarts se calculent à partir de deux profils.",
    ),
    (
        "comparer.ecarts.titre",
        "{nom} comparé à {reference}",
    ),
    (
        "comparer.ecarts.identiques",
        "Profils identiques\u{202f}: aucun écart sur les 35 937 valeurs RVB.",
    ),
    (
        "comparer.ecarts.memes_mesures",
        "Mêmes mesures\u{202f}: seul le logiciel de calcul change.",
    ),
    (
        "comparer.ecarts.refuse",
        "Pas d’écarts\u{202f}: ce profil, ou le profil de référence, n’est pas comparé.",
    ),
    (
        "comparer.ecarts.grille",
        "Sur 35 937 valeurs RVB",
    ),
    (
        "comparer.ecarts.moyenne",
        "Moyenne",
    ),
    (
        "comparer.ecarts.mediane",
        "Médiane",
    ),
    (
        "comparer.ecarts.p95",
        "p95",
    ),
    (
        "comparer.ecarts.max",
        "Max",
    ),
    (
        "comparer.ecarts.sup_1",
        "Au-delà de 1 ΔE00",
    ),
    (
        "comparer.ecarts.sup_2",
        "Au-delà de 2 ΔE00",
    ),
    (
        "comparer.ecarts.part",
        "{part}\u{202f}%",
    ),
    (
        "comparer.ecarts.zones",
        "Par zone",
    ),
    (
        "comparer.ecarts.zone",
        "Zone",
    ),
    (
        "comparer.ecarts.nombre",
        "Valeurs",
    ),
    (
        "comparer.ecarts.pires",
        "Les 10 écarts les plus forts",
    ),
    (
        "comparer.ecarts.rvb",
        "RVB",
    ),
    (
        "comparer.ecarts.lab_reference",
        "L*a*b* de référence",
    ),
    (
        "comparer.ecarts.lab_compare",
        "L*a*b* comparé",
    ),
    (
        "comparer.ecarts.ecart",
        "ΔE00",
    ),
    (
        "comparer.ecarts.primaires",
        "Couleurs de base",
    ),
    (
        "comparer.ecarts.fidelites",
        "Fidélités côte à côte",
    ),
    (
        "comparer.methode",
        "Méthode",
    ),
    (
        "comparer.methode.resume",
        "Méthode\u{202f}: ce qui est calculé, ce qui est approché",
    ),
    (
        "comparer.methode.calcule.titre",
        "Calculé",
    ),
    (
        "comparer.methode.calcule",
        "Valeurs lues dans les tables A2B1 des profils, en colorimétrie absolue (blanc du papier compris). Volume de la surface du cube RVB, à 33 points par arête. Limite du gamut et son volume par iccgamut (ArgyllCMS 3.5.0). Écarts ΔE00 sur 35 937 valeurs RVB.",
    ),
    (
        "comparer.methode.approche.titre",
        "Approché",
    ),
    (
        "comparer.methode.approche",
        "Coupe\u{202f}: section du gamut à la clarté choisie, lue tous les 2°. Chroma par teinte\u{202f}: la plus forte trouvée dans chaque tranche de 2°. Densité du noir tirée de Y, et non densité ISO status T. Pastilles d’écran\u{202f}: une couleur hors de l’écran est ramenée, et marquée «\u{202f}approchée\u{202f}».",
    ),
    (
        "comparer.methode.volume",
        "Le volume dit l’étendue des couleurs d’un profil, pas sa qualité.",
    ),
    (
        "comparer.methode.fidelite",
        "La fidélité se mesure sur les plages qui ont servi au calcul\u{202f}: elle avantage le profil qui les suit au plus près. Pour savoir lequel prédit le mieux un nouveau tirage, il faut mesurer une mire de contrôle (vérification, prévue).",
    ),
    (
        "comparer.provenance.origine",
        "Origine",
    ),
    (
        "comparer.provenance.version",
        "Version ICC",
    ),
    (
        "comparer.provenance.description",
        "Description",
    ),
    (
        "comparer.provenance.aucune",
        "aucune",
    ),
    (
        "comparer.provenance.grille",
        "Grille A2B1",
    ),
    (
        "comparer.provenance.grille.valeur",
        "{n} points par arête",
    ),
    (
        "comparer.provenance.mesures",
        "Mesures embarquées",
    ),
    (
        "comparer.provenance.mesures.valeur",
        "{n} plages",
    ),
    (
        "comparer.provenance.condition",
        "Condition de mesure",
    ),
    (
        "comparer.provenance.condition.inconnue",
        "inconnue",
    ),
    (
        "comparer.provenance.supposee",
        "supposée",
    ),
    (
        "comparer.provenance.rvb",
        "Valeurs RVB",
    ),
    (
        "comparer.provenance.rvb_supposes",
        "échelle 0 à 255 supposée",
    ),
    (
        "comparer.provenance.lab",
        "L*a*b* des mesures",
    ),
    (
        "comparer.provenance.lab_calcule",
        "calculé par myiro-libre depuis les spectres",
    ),
    (
        "comparer.provenance.lab_lu",
        "lu dans le fichier",
    ),
    (
        "comparer.avance",
        "Avancé\u{202f}: commande et journal d’iccgamut",
    ),
    (
        "comparer.avance.commande",
        "Commande",
    ),
    (
        "comparer.avance.sans_commande",
        "Sans ArgyllCMS, aucune commande n’est lancée.",
    ),
    (
        "comparer.avance.journal",
        "Journal d’iccgamut, {nom}",
    ),
    (
        "comparer.avance.cause",
        "Message d’ArgyllCMS, {nom}",
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
    // Préparer une mire (ticket #33)
    (
        "mire.erreur.cote",
        "Côté de plage refusé\u{202f}: le MYIRO-1 lit des plages de 7\u{202f}mm au moins, et la bande doit tenir sur la page. Choisissez un autre côté.",
    ),
    (
        "mire.erreur.nombre_libre",
        "Nombre de plages refusé\u{202f}: choisissez de 24 à 3\u{202f}000 plages.",
    ),
    (
        "mire.erreur.ti1",
        "Les plages choisies par ArgyllCMS sont illisibles. Rien n’a été préparé. Relancez la préparation.",
    ),
    (
        "mire.erreur.nombre",
        "ArgyllCMS n’a pas rendu le nombre de plages demandé. Rien n’a été préparé. Relancez la préparation.",
    ),
    (
        "mire.erreur.generation",
        "ArgyllCMS n’a pas pu choisir les plages de la mire. Rien n’a été préparé.",
    ),
    (
        "mire.erreur.contraste",
        "Plages trop proches pour être lues\u{202f}: certaines ne se distinguent pas assez de leur voisine ou du blanc du papier. Rien n’a été préparé. Choisissez un plus grand nombre de plages.",
    ),
    (
        "mire.erreur.ecriture",
        "Les pages de la mire n’ont pas pu être écrites dans le dossier Documents\\myiro-libre\\Mires. Vérifiez qu’il reste de la place et que le dossier n’est pas protégé, puis recommencez.",
    ),
    (
        "mire.erreur.fichier_existant",
        "Un fichier du même nom existe déjà dans le dossier Documents\\myiro-libre\\Mires\u{202f}: il n’a pas été remplacé, et rien n’a été préparé. Déplacez ou renommez l’ancien fichier, puis recommencez.",
    ),
    (
        "mire.erreur.fd9_introuvable",
        "Mire RGB-2520 introuvable. Placez le fichier «\u{202f}RGB-2520 A3 - 1p.tif\u{202f}» dans le dossier mire-fd9 à côté de l’application. Le MYIRO-1 reste utilisable.",
    ),
    (
        "mire.erreur.fd9_autre",
        "Ce fichier n’est pas la mire RGB-2520 retenue\u{202f}: il a été modifié ou remplacé. Rien n’a été préparé. Reprenez le fichier d’origine, sans l’ouvrir ni l’enregistrer dans un autre logiciel.",
    ),
    (
        "mire.erreur.fd9_illisible",
        "La mire RGB-2520 est illisible ou n’a pas la forme attendue. Rien n’a été préparé. Reprenez le fichier d’origine.",
    ),
    (
        "mire.erreur.fd9_fixe",
        "La mire du FD-9 est RGB-2520, sur une page A3\u{202f}: ni taille, ni format, ni côté à choisir.",
    ),
    ("mire.ouvrir", "Préparer une mire"),
    ("mire.titre", "Préparer une mire"),
    ("mire.instrument", "Instrument"),
    ("mire.instrument.fd9", "FD-9"),
    ("mire.instrument.myiro1", "MYIRO-1"),
    ("mire.fd9.carte", "Mire RGB-2520 · 2\u{202f}520 plages · 1 page A3"),
    (
        "mire.myiro1.carte",
        "Pages en bandes, taille et format au choix",
    ),
    ("mire.fd9.attendu", "Fichier attendu\u{202f}: {chemin}"),
    ("mire.taille", "Taille"),
    ("mire.taille.nom.rapide", "Rapide"),
    ("mire.taille.nom.standard", "Standard"),
    ("mire.taille.nom.approfondi", "Approfondi"),
    ("mire.plages", "{n} plages"),
    ("mire.pages.une", "1 page"),
    ("mire.pages.plusieurs", "{n} pages"),
    ("mire.format", "Format"),
    ("mire.format.a4", "A4"),
    ("mire.format.a3", "A3"),
    ("mire.avance", "Avancé"),
    ("mire.avance.resume", "{cote}\u{202f}mm, {plages} plages"),
    ("mire.cote", "Côté des plages, en millimètres"),
    (
        "mire.cote.limites",
        "{min}\u{202f}mm au moins (manuel du MYIRO-1), et la bande doit tenir sur la page.",
    ),
    (
        "mire.nombre",
        "Nombre de plages (vide pour garder la taille choisie)",
    ),
    ("mire.nombre.limites", "De 24 à 3\u{202f}000 plages."),
    (
        "mire.saisie.nombre",
        "Saisie refusée\u{202f}: écrivez un nombre, par exemple 8 ou 8,5. Rien n’est lancé.",
    ),
    (
        "mire.saisie.entier",
        "Saisie refusée\u{202f}: écrivez un nombre entier de plages, par exemple 500. Rien n’est lancé.",
    ),
    ("mire.commande", "Commande équivalente"),
    ("mire.preparer", "Préparer la mire"),
    ("mire.fait", "Mire préparée et rangée avec cette résolution."),
    (
        "mire.deja_preparee",
        "Mire déjà préparée le {date}, pas encore mesurée\u{202f}: ses fichiers sont repris, sans doublon.",
    ),
    ("mire.fichiers", "Fichiers à imprimer"),
    ("mire.ouvrir_dossier", "Ouvrir le dossier"),
    (
        "mire.rappel",
        "Imprimez à 100\u{202f}%, sans gestion des couleurs (dans le pilote et dans le logiciel d’impression).",
    ),
    ("mire.preparees", "Mires préparées"),
    (
        "mire.preparees.vide",
        "Aucune mire préparée pour cette résolution.",
    ),
];

const ANGLAIS: Catalogue = &[
    // Window title, with `--bibliotheque <folder>`
    ("fenetre.titre.essai", "myiro-libre — test library"),
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
    (
        "profil.erreur.outil_manquant",
        "A tool of ArgyllCMS is missing from the argyll folder next to the application. Replace this folder with a complete copy of ArgyllCMS 3.5.0, then restart the application.",
    ),
    (
        "comparer.refus.en_tete",
        "This file is not a readable ICC profile.",
    ),
    (
        "comparer.refus.pas_rvb",
        "This profile is not an RGB profile. CMYK comparison is not planned yet.",
    ),
    (
        "comparer.refus.sans_table",
        "This profile has no colorimetric table (A2B1): it may be a display profile. It is not compared.",
    ),
    (
        "comparer.refus.structure",
        "The colorimetric table of this profile has a structure myiro-libre does not read yet ({detail}). It is not compared.",
    ),
    (
        "comparer.refus.tronque",
        "This profile is incomplete: a part ({detail}) goes beyond the end of the file. It is not compared.",
    ),
    (
        "comparer.refus.sans_blanc",
        "This profile does not give the media white: its absolute colours cannot be computed. It is not compared.",
    ),
    (
        "comparer.refus.sans_table_inverse",
        "This profile has no readable inverse table (B2A1): which colours the printer would render from the image cannot be computed. This stays unknown.",
    ),
    (
        "comparer.refus.trop_de_profils",
        "At most four profiles are compared. Remove one, then start again.",
    ),
    (
        "comparer.refus.fichier_ferme",
        "This file is no longer open. Open it again.",
    ),
    (
        "comparer.refus.aucun_profil",
        "Choose at least one profile to compare.",
    ),
    (
        "comparer.refus.profil_inconnu",
        "This profile is no longer in the library. Choose another one from the list.",
    ),
    (
        "comparer.refus.fichier_illisible",
        "This file cannot be read. Check that it still exists, then open it again.",
    ),
    (
        "comparer.refus.fichier_trop_grand",
        "This file is far too large for an ICC profile: it is not opened.",
    ),
    (
        "comparer.image.refus.en_tete",
        "The profile embedded in this image is not a readable ICC profile ({detail}).",
    ),
    (
        "comparer.image.refus.pas_rvb",
        "The profile embedded in this image is a CMYK profile. CMYK images are planned for a second stage.",
    ),
    (
        "comparer.image.refus.sans_conversion",
        "The profile embedded in this image does not say which colours its pixels ask for: it has no A2B table, no matrix and curves, no grey curve.",
    ),
    (
        "comparer.image.refus.structure",
        "The profile embedded in this image has a structure myiro-libre does not read yet ({detail}).",
    ),
    (
        "comparer.image.refus.tronque",
        "The profile embedded in this image is incomplete: a part ({detail}) goes beyond the end of the profile.",
    ),
    (
        "comparer.image.refus.illisible",
        "The profile embedded in this image is damaged or incomplete: it cannot be read ({detail}).",
    ),
    (
        "image.ouvrir",
        "Add an image",
    ),
    (
        "image.filtre",
        "JPEG, PNG or TIFF images",
    ),
    (
        "image.traiter_srvb",
        "Treat as sRGB",
    ),
    (
        "image.refus.cmjn",
        "This image is in CMYK. Comparing CMYK images will come in a second stage. Open an RGB version of the photo.",
    ),
    (
        "image.refus.profil",
        "The profile embedded in the image cannot be read ({detail}). Choose “Treat as sRGB” if the photo comes from a camera or a website in sRGB.",
    ),
    (
        "image.refus.trop_grande",
        "This image is too large to open (more than 120 million pixels, or a file over 1 GB). Open a reduced version of the photo.",
    ),
    (
        "image.refus.illisible",
        "This file cannot be read as an image. Check that it still exists and opens in another program, then open it again.",
    ),
    (
        "image.refus.format",
        "This image format is not supported. Open the photo as JPEG, PNG or TIFF.",
    ),
    (
        "image.refus.aucune",
        "No image is open. Add one, then start again.",
    ),
    (
        "image.refus.sans_ecarts",
        "The differences for this profile are not computed. Run the image analysis again.",
    ),
    (
        "image.annulee",
        "Image analysis cancelled.",
    ),
    (
        "image.erreur.calcul",
        "The computation for this profile stopped on a myiro-libre error. This stays unknown.",
    ),
    (
        "image.profil.embarque",
        "Image profile: {nom}",
    ),
    (
        "image.profil.sans_nom",
        "unnamed embedded profile",
    ),
    (
        "image.profil.ecartee",
        "Table {table} of the image profile not used: {raison}",
    ),
    (
        "image.profil.srvb",
        "Image profile: sRGB (no embedded profile)",
    ),
    (
        "image.profil.suppose",
        "sRGB, assumed: chosen by you",
    ),
    (
        "image.approche",
        "Computed on 1 pixel in {n}",
    ),
    (
        "image.seize_bits",
        "16-bit values reduced to 8 bits",
    ),
    (
        "image.titre",
        "Image",
    ),
    (
        "image.aide",
        "Open a customer photo in RGB (JPEG, PNG or TIFF): for each displayed profile, the page shows the pixels the printer will not be able to reproduce. The image stays in memory for the comparison only; it is neither stored nor copied.",
    ),
    (
        "image.remplacer",
        "Open another image",
    ),
    (
        "image.retirer",
        "Remove the image",
    ),
    (
        "image.en_cours",
        "Analysing the image…",
    ),
    (
        "image.annuler",
        "Cancel",
    ),
    (
        "image.relancer",
        "Analyse again",
    ),
    (
        "image.taille",
        "Analysis image of {l} × {h} pixels, taken from a {lo} × {ho} pixel photo",
    ),
    (
        "image.surimpression.profil",
        "Overlay profile",
    ),
    (
        "image.surimpression.inconnue",
        "No overlay for this profile: {raison}",
    ),
    (
        "image.seuil.valeur",
        "Alert threshold {s} ΔE00",
    ),
    (
        "image.affichage",
        "Show",
    ),
    (
        "image.affichage.image",
        "Image",
    ),
    (
        "image.affichage.les_deux",
        "Image and alert",
    ),
    (
        "image.affichage.alerte",
        "Alert only",
    ),
    (
        "nuage.bascule",
        "Image colours",
    ),
    (
        "nuage.aide",
        "Each point is the average of a group of close colours of the image, placed on the paper white of {profil}. It takes the alert colour as soon as one of its colours exceeds {s} ΔE00.",
    ),
    (
        "nuage.indisponible",
        "The image colours appear here once an image is analysed against the profile chosen for the overlay.",
    ),
    (
        "nuage.aria",
        "Image colours: {n} points, {a} of them beyond the alert threshold.",
    ),
    (
        "nuage.compte",
        "Image colours in the slice (L* {l} ± {d}): {n} points, {a} of them beyond the alert threshold.",
    ),
    (
        "nuage.legende",
        "Image colours, L* {l} ± {d}",
    ),
    (
        "nuage.legende.alerte",
        "Image colour beyond the alert threshold",
    ),
    (
        "image.legende",
        "Difference of at most {s} ΔE00, {profil}, without black point compensation. In green, beyond.",
    ),
    (
        "image.canvas.aria",
        "{p}% of the image beyond {s} ΔE00 with {profil}. Click the image to see the colour of a point and its printed colour.",
    ),
    (
        "image.chiffres",
        "Image figures",
    ),
    (
        "image.chiffres.part",
        "Share of the image beyond {s} ΔE00",
    ),
    (
        "image.classe.leger",
        "Slight, 2 to 5 ΔE00",
    ),
    (
        "image.classe.moyen",
        "Medium, 5 to 10 ΔE00",
    ),
    (
        "image.classe.fort",
        "Strong, over 10 ΔE00",
    ),
    (
        "image.chiffres.teintes",
        "Most affected hues",
    ),
    (
        "image.chiffres.inconnus",
        "Pixels with an unknown difference",
    ),
    (
        "image.chiffres.couleurs",
        "Distinct colours",
    ),
    (
        "image.chiffres.raison",
        "Reason",
    ),
    (
        "image.valeur.pourcent",
        "{p}%",
    ),
    (
        "image.teinte.valeur",
        "{nom} {p}%",
    ),
    (
        "image.teintes.aucune",
        "none",
    ),
    (
        "image.inconnus.valeur",
        "{n} pixels ({p}%), never counted as good",
    ),
    (
        "image.inconnus.aucun",
        "none",
    ),
    (
        "image.teinte.rouge",
        "Reds",
    ),
    (
        "image.teinte.jaune",
        "Yellows",
    ),
    (
        "image.teinte.vert",
        "Greens",
    ),
    (
        "image.teinte.cyan",
        "Cyans",
    ),
    (
        "image.teinte.bleu",
        "Blues",
    ),
    (
        "image.teinte.magenta",
        "Magentas",
    ),
    (
        "image.teinte.gris",
        "Greys (C* < 5)",
    ),
    (
        "image.verdict",
        "{p}% of the image is printable on this paper with this profile",
    ),
    (
        "image.verdict.plus_de",
        "over {p}",
    ),
    (
        "image.verdict.moins_de",
        "under {p}",
    ),
    (
        "image.verdict.inconnus",
        "; {p}% unknown",
    ),
    (
        "image.verdict.inconnu",
        "Printable share unknown with this profile",
    ),
    (
        "image.point_noir",
        "Black point compensation",
    ),
    (
        "image.point_noir.aide",
        "The image black is brought onto the deepest black of the paper, as the printer driver does in relative colorimetric with compensation. Changing this setting runs the analysis again.",
    ),
    (
        "image.legende.point_noir",
        "Difference of at most {s} ΔE00, {profil}, with black point compensation. In green, beyond.",
    ),
    (
        "image.rendu.point_noir",
        "Relative colorimetric, with black point compensation",
    ),
    (
        "image.rendu.relatif",
        "Relative colorimetric, without black point compensation",
    ),
    (
        "image.agrandir",
        "Enlarge",
    ),
    (
        "image.agrandie.titre",
        "Enlarged image",
    ),
    (
        "image.agrandie.fermer",
        "Close",
    ),
    (
        "image.agrandie.aide",
        "Click the image to see the colour of a point. Escape or a click beside the image to go back.",
    ),
    (
        "pastilles.pipette.visee",
        "Target colour after black point compensation: {lab}, {e} ΔE00 from the image colour",
    ),
    (
        "pastilles.pipette.visee_identique",
        "Black point compensation: the target colour is the image colour.",
    ),
    (
        "pastilles.aria.visee",
        "Target colour: L* {l}, a* {a}, b* {b}",
    ),
    (
        "image.pixel.hors_image",
        "This point is outside the image.",
    ),
    (
        "image.pixel.transparent",
        "This point of the image is transparent: it has no colour to print.",
    ),
    (
        "image.pixel.inconnu",
        "The difference for this colour could not be calculated.",
    ),
    (
        "pastilles.titre",
        "Out-of-gamut colours",
    ),
    (
        "pastilles.lecture",
        "Left, the image colour; right, the colour printed with the chosen profile; below, the difference and the share of the image. Most present first.",
    ),
    (
        "pastilles.aucune",
        "None of the most present image colours is beyond {s} ΔE00 with {profil}.",
    ),
    (
        "pastilles.precaution",
        "Approximate on-screen colours (uncalibrated screen).",
    ),
    (
        "pastilles.ecart",
        "{e} ΔE00",
    ),
    (
        "pastilles.part",
        "{p}%",
    ),
    (
        "pastilles.aria.image",
        "Image colour: L* {l}, a* {a}, b* {b}",
    ),
    (
        "pastilles.aria.imprimee",
        "Printed colour: L* {l}, a* {a}, b* {b}",
    ),
    (
        "pastilles.image",
        "Image",
    ),
    (
        "pastilles.imprimee",
        "Printed",
    ),
    (
        "pastilles.lab",
        "L* {l}  a* {a}  b* {b}",
    ),
    (
        "pastilles.pipette.titre",
        "Colour of the clicked point",
    ),
    (
        "pastilles.pipette.aide",
        "Click the image to see the colour of a point and its printed colour.",
    ),
    (
        "pastilles.pipette.au_dela",
        "Beyond the {s} ΔE00 alert threshold",
    ),
    (
        "pastilles.pipette.en_deca",
        "Within the {s} ΔE00 alert threshold",
    ),
    (
        "pastilles.pipette.inconnu",
        "Printed colour unknown with {profil}: {raison}",
    ),
    (
        "comparer.zone.gris",
        "Greys",
    ),
    (
        "comparer.zone.ombres",
        "Shadows",
    ),
    (
        "comparer.zone.clairs",
        "Highlights",
    ),
    (
        "comparer.zone.saturee.rouge",
        "Saturated reds",
    ),
    (
        "comparer.zone.saturee.jaune",
        "Saturated yellows",
    ),
    (
        "comparer.zone.saturee.vert",
        "Saturated greens",
    ),
    (
        "comparer.zone.saturee.cyan",
        "Saturated cyans",
    ),
    (
        "comparer.zone.saturee.bleu",
        "Saturated blues",
    ),
    (
        "comparer.zone.saturee.magenta",
        "Saturated magentas",
    ),
    (
        "comparer.zone.moyenne.rouge",
        "Medium reds",
    ),
    (
        "comparer.zone.moyenne.jaune",
        "Medium yellows",
    ),
    (
        "comparer.zone.moyenne.vert",
        "Medium greens",
    ),
    (
        "comparer.zone.moyenne.cyan",
        "Medium cyans",
    ),
    (
        "comparer.zone.moyenne.bleu",
        "Medium blues",
    ),
    (
        "comparer.zone.moyenne.magenta",
        "Medium magentas",
    ),
    (
        "comparer.fidelite.sans_mesures",
        "Unknown: this profile does not embed its measurements.",
    ),
    (
        "comparer.fidelite.illisible",
        "Unknown: the measurements embedded in this profile cannot be read.",
    ),
    (
        "comparer.fidelite.echelle_spectre",
        "Unknown: the highest value of the embedded spectra is between 1.5 and 2. It is not known whether they are a reflectance from 0 to 1 or a percentage, and nothing is guessed.",
    ),
    (
        "comparer.fidelite.rvb_inconnus",
        "Unknown: the embedded measurements do not say which RGB values were printed, or on which scale.",
    ),
    (
        "comparer.fidelite.rvb_hors_echelle",
        "Unknown: a patch of the embedded measurements has an RGB value outside the expected scale.",
    ),
    (
        "comparer.fidelite.lab_inconnu",
        "Unknown: a patch of the embedded measurements has neither Lab nor a usable spectrum.",
    ),
    (
        "comparer.limite.sans_argyll",
        "The gamut boundary needs ArgyllCMS 3.5.0, which cannot be found next to the application. The raw surface is shown, and the rest of the comparison is computed.",
    ),
    (
        "comparer.limite.sans_iccgamut",
        "The iccgamut tool of ArgyllCMS is missing: the gamut boundary is not computed, the rest of the comparison is. Replace the argyll folder with a complete copy of ArgyllCMS 3.5.0.",
    ),
    (
        "comparer.limite.argyll_version",
        "ArgyllCMS {version} was found, but only version 3.5.0 is validated: the gamut boundary is not computed, the rest of the comparison is.",
    ),
    (
        "comparer.limite.argyll_illisible",
        "ArgyllCMS does not answer as expected: the gamut boundary is not computed, the rest of the comparison is.",
    ),
    (
        "comparer.limite.fichier",
        "A working file of iccgamut could not be read or written: the gamut boundary is not computed, the rest of the comparison is. Try again.",
    ),
    (
        "comparer.limite.illisible",
        "The gamut boundary written by iccgamut cannot be read: it is not shown, the rest of the comparison is computed.",
    ),
    (
        "comparer.limite.interrompue",
        "iccgamut stopped before the end without reporting an error: the gamut boundary is not computed, the rest of the comparison is. Run the comparison again.",
    ),
    (
        "comparer.erreur.limite",
        "Computing the gamut boundary failed in ArgyllCMS (iccgamut): it is not shown, the rest of the comparison is computed.",
    ),
    (
        "comparer.origine.bibliotheque",
        "Library: {chemin}, profile of {date}",
    ),
    (
        "comparer.origine.fichier",
        "File: {nom} (opened for the comparison, not in the library)",
    ),
    (
        "comparer.inconnu",
        "unknown",
    ),
    (
        "comparer.vue3d.aria",
        "Gamuts in 3D (arrow keys to rotate, + and − to zoom, 0 to return to the starting view): {profils}.",
    ),
    (
        "comparer.vue3d.aria.profil",
        "{nom}, volume {volume} cubic L*a*b* units, paper white L* {blanc}",
    ),
    (
        "comparer.vue3d.aria.vide",
        "Gamuts in 3D: no profile shown.",
    ),
    (
        "comparer.vue3d.bulle",
        "{nom}: L* {l}, a* {a}, b* {b}",
    ),
    (
        "comparer.vue3d.blanc",
        "Paper white L* {l}",
    ),
    (
        "comparer.graphique.teinte",
        "Hue h (°)",
    ),
    (
        "comparer.graphique.chroma",
        "Chroma C*",
    ),
    (
        "comparer.graphique.clarte",
        "Lightness L*",
    ),
    (
        "comparer.graphique.position",
        "Position along the path (0 to 255)",
    ),
    (
        "comparer.graphique.mesures",
        "Patches measured on this path",
    ),
    (
        "comparer.graphique.valeurs",
        "Values",
    ),
    (
        "comparer.graphique.plage_mesuree",
        "{position} (measured patch)",
    ),
    (
        "comparer.graphique.teintes.aria",
        "Maximum chroma C* by hue, from 0 to 360°: {profils}. The values are in the “Values” table.",
    ),
    (
        "comparer.graphique.coupe.aria",
        "Gamut slice at lightness L* {l}, in the a*b* plane: {profils}. The values are in the “Values” table.",
    ),
    (
        "comparer.graphique.chemin.l.aria",
        "Lightness L* along the RGB cube path: {profils}. The values are in the “Values” table.",
    ),
    (
        "comparer.graphique.chemin.c.aria",
        "Chroma C* along the RGB cube path: {profils}. The values are in the “Values” table.",
    ),
    (
        "comparer.titre",
        "Compare profiles",
    ),
    (
        "comparer.aide",
        "One to four RGB profiles, stored in the library or opened from a file. The first one is the reference for the differences.",
    ),
    (
        "comparer.profils",
        "Compared profiles",
    ),
    (
        "comparer.ajouter",
        "Add a profile",
    ),
    (
        "comparer.ajouter.chargement",
        "Reading the list of profiles…",
    ),
    (
        "comparer.ajouter.max",
        "Four profiles at most: remove one to add another.",
    ),
    (
        "comparer.fermer",
        "Close the comparison",
    ),
    (
        "comparer.en_cours",
        "Comparison in progress: the gamut boundary can take a few seconds per profile.",
    ),
    (
        "comparer.ouvrir",
        "Compare",
    ),
    (
        "comparer.ouvrir.actuel",
        "Compare with current",
    ),
    (
        "comparer.retirer",
        "Remove",
    ),
    (
        "comparer.reference",
        "reference",
    ),
    (
        "comparer.choix.titre",
        "Library profiles",
    ),
    (
        "comparer.choix.vide",
        "No profile stored in the library yet.",
    ),
    (
        "comparer.choix.actuel",
        "current",
    ),
    (
        "comparer.choix.fichier",
        "Open an .icc file",
    ),
    (
        "comparer.vide",
        "Add a profile to start the comparison.",
    ),
    (
        "comparer.chiffres",
        "Key figures",
    ),
    (
        "comparer.chiffres.blanc",
        "Paper white",
    ),
    (
        "comparer.chiffres.noir",
        "Black",
    ),
    (
        "comparer.chiffres.noir.valeur",
        "L* {l} · density ≈ {d}",
    ),
    (
        "comparer.chiffres.volume_surface",
        "Volume, raw surface",
    ),
    (
        "comparer.chiffres.volume_limite",
        "Volume, gamut boundary",
    ),
    (
        "comparer.chiffres.limite_indisponible",
        "not computed",
    ),
    (
        "comparer.chiffres.fidelite",
        "Fidelity to its measurements",
    ),
    (
        "comparer.chiffres.fidelite.valeur",
        "mean {moyenne}, p95 {p95}, max {max} ΔE00 over {plages} patches",
    ),
    (
        "comparer.chiffres.repetabilite",
        "Measurement repeatability",
    ),
    (
        "comparer.chiffres.repetabilite.valeur",
        "mean {moyenne}, max {max} ΔE00 ({couleurs} colours, {paires} pairs)",
    ),
    (
        "comparer.chiffres.repetabilite.inconnue",
        "unknown: no patch measured twice",
    ),
    (
        "comparer.valeur.lab",
        "L* {l}, a* {a}, b* {b}",
    ),
    (
        "comparer.valeur.volume",
        "{volume} cubic L*a*b* units",
    ),
    (
        "comparer.volume",
        "Gamut in 3D",
    ),
    (
        "comparer.volume.affiches",
        "Profiles shown",
    ),
    (
        "comparer.volume.couleur",
        "Colour",
    ),
    (
        "comparer.volume.couleur.serie",
        "By profile",
    ),
    (
        "comparer.volume.couleur.reelle",
        "Real colours",
    ),
    (
        "comparer.volume.couleur.aide",
        "In real colours, only the first profile shown is dressed in its colours: the reference profile, when it is ticked. The other profiles shown are drawn as a wireframe, in their profile colour, so they can be told apart.",
    ),
    (
        "comparer.volume.forme",
        "Shape",
    ),
    (
        "comparer.volume.forme.limite",
        "Gamut boundary",
    ),
    (
        "comparer.volume.forme.surface",
        "Raw surface",
    ),
    (
        "comparer.volume.rendu",
        "Rendering",
    ),
    (
        "comparer.volume.rendu.opaque",
        "Opaque",
    ),
    (
        "comparer.volume.rendu.transparent",
        "Transparent",
    ),
    (
        "comparer.volume.aide",
        "Drag to rotate, wheel to zoom; with the keyboard, arrows, + and −, 0 to reset.",
    ),
    (
        "comparer.volume.surface_montree",
        "{nom}: raw surface shown. {raison}",
    ),
    (
        "comparer.teintes",
        "Maximum chroma by hue",
    ),
    (
        "comparer.coupe",
        "Constant lightness slice",
    ),
    (
        "comparer.coupe.clarte",
        "Lightness L*",
    ),
    (
        "comparer.chemin",
        "Curves along a path of the RGB cube",
    ),
    (
        "comparer.chemin.depart",
        "Start",
    ),
    (
        "comparer.chemin.arrivee",
        "End",
    ),
    (
        "comparer.primaires",
        "Primary colours",
    ),
    (
        "comparer.primaire.blanc",
        "White",
    ),
    (
        "comparer.primaire.noir",
        "Black",
    ),
    (
        "comparer.primaire.rouge",
        "Red",
    ),
    (
        "comparer.primaire.vert",
        "Green",
    ),
    (
        "comparer.primaire.bleu",
        "Blue",
    ),
    (
        "comparer.primaire.cyan",
        "Cyan",
    ),
    (
        "comparer.primaire.magenta",
        "Magenta",
    ),
    (
        "comparer.primaire.jaune",
        "Yellow",
    ),
    (
        "comparer.primaires.couleur",
        "Colour",
    ),
    (
        "comparer.primaires.profil",
        "Profile",
    ),
    (
        "comparer.primaires.pastille",
        "Swatch",
    ),
    (
        "comparer.primaires.approchee",
        "approximate",
    ),
    (
        "comparer.primaires.sans_teinte",
        "—",
    ),
    (
        "comparer.ecarts",
        "Differences from the reference profile",
    ),
    (
        "comparer.ecarts.deux",
        "Differences are computed from two profiles.",
    ),
    (
        "comparer.ecarts.titre",
        "{nom} compared with {reference}",
    ),
    (
        "comparer.ecarts.identiques",
        "Identical profiles: no difference over the 35,937 RGB values.",
    ),
    (
        "comparer.ecarts.memes_mesures",
        "Same measurements: only the computing software changes.",
    ),
    (
        "comparer.ecarts.refuse",
        "No differences: this profile, or the reference profile, is not compared.",
    ),
    (
        "comparer.ecarts.grille",
        "Over 35,937 RGB values",
    ),
    (
        "comparer.ecarts.moyenne",
        "Mean",
    ),
    (
        "comparer.ecarts.mediane",
        "Median",
    ),
    (
        "comparer.ecarts.p95",
        "p95",
    ),
    (
        "comparer.ecarts.max",
        "Max",
    ),
    (
        "comparer.ecarts.sup_1",
        "Above 1 ΔE00",
    ),
    (
        "comparer.ecarts.sup_2",
        "Above 2 ΔE00",
    ),
    (
        "comparer.ecarts.part",
        "{part}%",
    ),
    (
        "comparer.ecarts.zones",
        "By area",
    ),
    (
        "comparer.ecarts.zone",
        "Area",
    ),
    (
        "comparer.ecarts.nombre",
        "Values",
    ),
    (
        "comparer.ecarts.pires",
        "The 10 largest differences",
    ),
    (
        "comparer.ecarts.rvb",
        "RGB",
    ),
    (
        "comparer.ecarts.lab_reference",
        "Reference L*a*b*",
    ),
    (
        "comparer.ecarts.lab_compare",
        "Compared L*a*b*",
    ),
    (
        "comparer.ecarts.ecart",
        "ΔE00",
    ),
    (
        "comparer.ecarts.primaires",
        "Primary colours",
    ),
    (
        "comparer.ecarts.fidelites",
        "Fidelities side by side",
    ),
    (
        "comparer.methode",
        "Method",
    ),
    (
        "comparer.methode.resume",
        "Method: what is computed, what is approximated",
    ),
    (
        "comparer.methode.calcule.titre",
        "Computed",
    ),
    (
        "comparer.methode.calcule",
        "Values read from the A2B1 tables of the profiles, in absolute colorimetry (paper white included). Volume of the RGB cube surface, 33 points per edge. Gamut boundary and its volume by iccgamut (ArgyllCMS 3.5.0). ΔE00 differences over 35,937 RGB values.",
    ),
    (
        "comparer.methode.approche.titre",
        "Approximated",
    ),
    (
        "comparer.methode.approche",
        "Slice: section of the gamut at the chosen lightness, read every 2°. Chroma by hue: the highest found in each 2° step. Black density derived from Y, not an ISO status T density. Screen swatches: a colour outside the screen is brought back, and marked “approximate”.",
    ),
    (
        "comparer.methode.volume",
        "The volume tells the extent of a profile’s colours, not its quality.",
    ),
    (
        "comparer.methode.fidelite",
        "Fidelity is measured on the patches used for the computation: it favours the profile that follows them most closely. To know which one best predicts a new print, a control chart must be measured (verification, planned).",
    ),
    (
        "comparer.provenance.origine",
        "Origin",
    ),
    (
        "comparer.provenance.version",
        "ICC version",
    ),
    (
        "comparer.provenance.description",
        "Description",
    ),
    (
        "comparer.provenance.aucune",
        "none",
    ),
    (
        "comparer.provenance.grille",
        "A2B1 grid",
    ),
    (
        "comparer.provenance.grille.valeur",
        "{n} points per edge",
    ),
    (
        "comparer.provenance.mesures",
        "Embedded measurements",
    ),
    (
        "comparer.provenance.mesures.valeur",
        "{n} patches",
    ),
    (
        "comparer.provenance.condition",
        "Measurement condition",
    ),
    (
        "comparer.provenance.condition.inconnue",
        "unknown",
    ),
    (
        "comparer.provenance.supposee",
        "assumed",
    ),
    (
        "comparer.provenance.rvb",
        "RGB values",
    ),
    (
        "comparer.provenance.rvb_supposes",
        "0 to 255 scale assumed",
    ),
    (
        "comparer.provenance.lab",
        "Measurement L*a*b*",
    ),
    (
        "comparer.provenance.lab_calcule",
        "computed by myiro-libre from the spectra",
    ),
    (
        "comparer.provenance.lab_lu",
        "read from the file",
    ),
    (
        "comparer.avance",
        "Advanced: iccgamut command and log",
    ),
    (
        "comparer.avance.commande",
        "Command",
    ),
    (
        "comparer.avance.sans_commande",
        "Without ArgyllCMS, no command is run.",
    ),
    (
        "comparer.avance.journal",
        "iccgamut log, {nom}",
    ),
    (
        "comparer.avance.cause",
        "ArgyllCMS message, {nom}",
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
    // Prepare a chart (ticket #33)
    (
        "mire.erreur.cote",
        "Patch size refused: the MYIRO-1 reads patches of at least 7 mm, and the strip must fit on the page. Choose another size.",
    ),
    (
        "mire.erreur.nombre_libre",
        "Number of patches refused: choose from 24 to 3,000 patches.",
    ),
    (
        "mire.erreur.ti1",
        "The patches chosen by ArgyllCMS cannot be read. Nothing was prepared. Prepare the chart again.",
    ),
    (
        "mire.erreur.nombre",
        "ArgyllCMS did not return the number of patches requested. Nothing was prepared. Prepare the chart again.",
    ),
    (
        "mire.erreur.generation",
        "ArgyllCMS could not choose the patches of the chart. Nothing was prepared.",
    ),
    (
        "mire.erreur.contraste",
        "Patches too close to be read: some do not stand out enough from their neighbour or from the paper white. Nothing was prepared. Choose a larger number of patches.",
    ),
    (
        "mire.erreur.ecriture",
        "The chart pages could not be written to the Documents\\myiro-libre\\Mires folder. Check that there is space left and that the folder is not protected, then try again.",
    ),
    (
        "mire.erreur.fichier_existant",
        "A file with the same name already exists in the Documents\\myiro-libre\\Mires folder: it was not replaced, and nothing was prepared. Move or rename the old file, then try again.",
    ),
    (
        "mire.erreur.fd9_introuvable",
        "RGB-2520 chart not found. Place the file “RGB-2520 A3 - 1p.tif” in the mire-fd9 folder next to the application. The MYIRO-1 can still be used.",
    ),
    (
        "mire.erreur.fd9_autre",
        "This file is not the selected RGB-2520 chart: it has been modified or replaced. Nothing was prepared. Use the original file, without opening or saving it in another program.",
    ),
    (
        "mire.erreur.fd9_illisible",
        "The RGB-2520 chart cannot be read or does not have the expected layout. Nothing was prepared. Use the original file.",
    ),
    (
        "mire.erreur.fd9_fixe",
        "The FD-9 chart is RGB-2520, on one A3 page: no size, format or patch size to choose.",
    ),
    ("mire.ouvrir", "Prepare a chart"),
    ("mire.titre", "Prepare a chart"),
    ("mire.instrument", "Instrument"),
    ("mire.instrument.fd9", "FD-9"),
    ("mire.instrument.myiro1", "MYIRO-1"),
    ("mire.fd9.carte", "RGB-2520 chart · 2,520 patches · 1 A3 page"),
    ("mire.myiro1.carte", "Pages in strips, size and format to choose"),
    ("mire.fd9.attendu", "Expected file: {chemin}"),
    ("mire.taille", "Size"),
    ("mire.taille.nom.rapide", "Quick"),
    ("mire.taille.nom.standard", "Standard"),
    ("mire.taille.nom.approfondi", "Thorough"),
    ("mire.plages", "{n} patches"),
    ("mire.pages.une", "1 page"),
    ("mire.pages.plusieurs", "{n} pages"),
    ("mire.format", "Format"),
    ("mire.format.a4", "A4"),
    ("mire.format.a3", "A3"),
    ("mire.avance", "Advanced"),
    ("mire.avance.resume", "{cote} mm, {plages} patches"),
    ("mire.cote", "Patch size, in millimetres"),
    (
        "mire.cote.limites",
        "At least {min} mm (MYIRO-1 manual), and the strip must fit on the page.",
    ),
    (
        "mire.nombre",
        "Number of patches (empty to keep the chosen size)",
    ),
    ("mire.nombre.limites", "From 24 to 3,000 patches."),
    (
        "mire.saisie.nombre",
        "Entry refused: write a number, for example 8 or 8.5. Nothing is started.",
    ),
    (
        "mire.saisie.entier",
        "Entry refused: write a whole number of patches, for example 500. Nothing is started.",
    ),
    ("mire.commande", "Equivalent command"),
    ("mire.preparer", "Prepare the chart"),
    ("mire.fait", "Chart prepared and stored with this resolution."),
    (
        "mire.deja_preparee",
        "Chart already prepared on {date}, not yet measured: its files are reused, no duplicate.",
    ),
    ("mire.fichiers", "Files to print"),
    ("mire.ouvrir_dossier", "Open the folder"),
    (
        "mire.rappel",
        "Print at 100%, without colour management (in the driver and in the printing software).",
    ),
    ("mire.preparees", "Prepared charts"),
    (
        "mire.preparees.vide",
        "No chart prepared for this resolution.",
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

    /// Chaque refus du profil d'une image (#80) a son texte, dans les deux
    /// langues.
    #[test]
    fn les_refus_d_un_profil_d_image_ont_leur_texte() {
        use gamut::{ErreurIcc, ProfilSource};
        let refus = [
            ErreurIcc::EnTete(String::new()),
            ErreurIcc::PasRvb {
                espace: "CMYK".into(),
            },
            ErreurIcc::SansTable,
            ErreurIcc::StructureNonPriseEnCharge {
                structure: "mAB".into(),
            },
            ErreurIcc::Tronque {
                balise: "kTRC".into(),
            },
        ];
        for erreur in &refus {
            let cle = ProfilSource::cle_refus(erreur);
            for langue in [Langue::Francais, Langue::Anglais] {
                assert_ne!(texte(langue, cle), cle, "{cle} manque");
            }
        }
    }

    /// Avec `--bibliotheque <dossier>`, le titre de la fenêtre dit que la
    /// bibliothèque ouverte est celle d'essai (#86).
    #[test]
    fn le_titre_de_la_fenetre_dit_la_bibliotheque_d_essai() {
        assert_eq!(
            texte(Langue::Francais, "fenetre.titre.essai"),
            "myiro-libre — bibliothèque d’essai"
        );
        assert_eq!(
            texte(Langue::Anglais, "fenetre.titre.essai"),
            "myiro-libre — test library"
        );
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
