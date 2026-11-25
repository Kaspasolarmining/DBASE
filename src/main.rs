// En mode graphique sous Windows, on utilise le sous-système « windows » pour
// NE PAS ouvrir de fenêtre console noire au lancement (l'appli n'a que des boîtes
// de dialogue). Sans effet sur Linux / en mode console.
#![cfg_attr(all(feature = "gui", target_os = "windows"), windows_subsystem = "windows")]

//! dbf_protection — déverrouille la colonne de protection `PASSWORD` d'un
//! fichier DBF (dBase IV), typiquement la table Siemens `SUBBLK`.
//!
//! Comportement :
//!   * on choisit le fichier `.dbf` dans l'explorateur (interface native) ;
//!   * la colonne traitée est TOUJOURS `PASSWORD` ;
//!   * toutes les valeurs de cette colonne sont remises à `0` (protection = non) ;
//!   * le fichier d'ENTRÉE est ÉCRASÉ par le résultat (modification sur place) ;
//!   * une copie de sécurité « .bak » de l'original est créée au préalable.
//!
//! Deux modes de compilation :
//!   * par défaut (feature `gui`)  : boîtes de dialogue natives via `rfd` ;
//!   * `--no-default-features`     : mode console (on passe le chemin en argument),
//!                                   pratique pour tester sans écran.

mod dbf;
use dbf::Dbf;

use std::error::Error;
use std::path::{Path, PathBuf};

/// Nom de la colonne de protection : figé, comme demandé.
const COLONNE: &str = "PASSWORD";

/// Cœur du traitement, commun aux deux modes (console / graphique).
///
/// Renvoie un message prêt à afficher, ou une erreur.
fn traiter(path: &Path) -> Result<String, Box<dyn Error>> {
    // 1) Charger le fichier choisi.
    let mut doc = Dbf::open(path)?;

    // 2) Vérifier que la colonne PASSWORD existe bien.
    let champ = doc.field(COLONNE).ok_or_else(|| {
        format!(
            "Colonne « {} » absente de ce fichier.\nColonnes présentes : {}",
            COLONNE,
            doc.fields.iter().map(|f| f.name.clone()).collect::<Vec<_>>().join(", ")
        )
    })?;

    // 3) Sauvegarde de sécurité : <fichier>.bak (on écrit l'original tel quel
    //    AVANT de l'écraser). On ne touche pas à l'éventuel fichier mémo .DBT.
    let mut bak: PathBuf = path.to_path_buf();
    let ext = bak.extension().and_then(|e| e.to_str()).unwrap_or("dbf").to_string();
    bak.set_extension(format!("{}.bak", ext));
    std::fs::write(&bak, &doc.bytes)?;

    // 4) Traitement : tout à 0.
    let modifies = doc.clear_numeric_to_zero(&champ);

    // 5) Écraser le fichier d'entrée avec le résultat.
    doc.save(path)?;

    Ok(format!(
        "Terminé ✅\n\n\
         Fichier      : {}\n\
         Colonne      : {}\n\
         Enregistrements : {}\n\
         Protections remises à 0 : {}\n\n\
         Sauvegarde de l'original : {}",
        path.display(),
        COLONNE,
        doc.record_count,
        modifies,
        bak.display()
    ))
}

// ───────────────────────────── Mode GRAPHIQUE ─────────────────────────────
#[cfg(feature = "gui")]
fn main() {
    use rfd::{FileDialog, MessageDialog, MessageLevel};

    // Explorateur de fichiers natif, filtré sur les .dbf, nom par défaut SUBBLK.DBF.
    let choix = FileDialog::new()
        .add_filter("Fichiers DBF", &["dbf", "DBF"])
        .set_file_name("SUBBLK.DBF")
        .set_title("Choisir le fichier SUBBLK.DBF à traiter")
        .pick_file();

    let chemin = match choix {
        Some(p) => p,
        None => return, // l'utilisateur a annulé : on quitte sans rien faire.
    };

    // Confirmation, car l'opération écrase le fichier d'origine.
    // En rfd 0.14, `show()` renvoie un `MessageDialogResult` (pas un booléen).
    let reponse = MessageDialog::new()
        .set_level(MessageLevel::Warning)
        .set_title("Confirmer")
        .set_description(format!(
            "Le fichier suivant va être MODIFIÉ SUR PLACE\n(une copie .bak sera créée) :\n\n{}\n\n\
             Colonne {} → 0 partout. Continuer ?",
            chemin.display(),
            COLONNE
        ))
        .set_buttons(rfd::MessageButtons::OkCancel)
        .show();
    if reponse != rfd::MessageDialogResult::Ok {
        return; // l'utilisateur a cliqué « Cancel »
    }

    match traiter(&chemin) {
        Ok(msg) => {
            MessageDialog::new()
                .set_level(MessageLevel::Info)
                .set_title("dbf_protection")
                .set_description(msg)
                .show();
        }
        Err(e) => {
            MessageDialog::new()
                .set_level(MessageLevel::Error)
                .set_title("Erreur")
                .set_description(format!("{}", e))
                .show();
        }
    }
}

// ───────────────────────────── Mode CONSOLE ──────────────────────────────
// Compilé seulement avec `--no-default-features` (sert aux tests sans écran).
#[cfg(not(feature = "gui"))]
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage (mode console) : {} <fichier.dbf>", args[0]);
        std::process::exit(2);
    }
    let msg = traiter(Path::new(&args[1]))?;
    println!("{}", msg);
    Ok(())
}
