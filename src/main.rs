// En mode graphique sous Windows, on utilise le sous-système « windows » pour
// NE PAS ouvrir de fenêtre console noire au lancement (l'appli n'a que des boîtes
// de dialogue). Sans effet sur Linux / en mode console.
#![cfg_attr(all(feature = "gui", target_os = "windows"), windows_subsystem = "windows")]

//! dbf_protection — retire la protection des blocs d'un PROJET Step7.
//!
//! Déroulement :
//!   1. l'utilisateur choisit un fichier projet `*.S7P` dans l'explorateur ;
//!   2. le programme part du dossier de ce projet et descend dans le
//!      sous-dossier `ombstx\offline` ;
//!   3. il traite TOUS les fichiers `SUBBLK.DBF` trouvés récursivement
//!      là-dessous : colonne `PASSWORD` remise à `0` (protection = non),
//!      fichier écrasé sur place, copie de sécurité `.bak` créée à côté ;
//!   4. à la fin, une boîte de dialogue « OK » affiche le nombre total de
//!      protections enlevées.
//!
//! Deux modes de compilation :
//!   * par défaut (feature `gui`)  : boîtes de dialogue natives via `rfd` ;
//!   * `--no-default-features`     : mode console (on passe le `.S7P` en argument),
//!                                   pratique pour tester sans écran.

mod dbf;
use dbf::Dbf;

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

/// Colonne de protection : figée.
const COLONNE: &str = "PASSWORD";
/// Nom des fichiers à traiter : figé.
const CIBLE: &str = "SUBBLK.DBF";

/// Bilan d'un traitement de projet, pour l'affichage final.
#[derive(Default)]
struct Rapport {
    fichiers: usize,          // nombre de SUBBLK.DBF traités
    protections: usize,       // total de protections remises à 0
    details: Vec<String>,     // une ligne par fichier
    erreurs: Vec<String>,     // fichiers en erreur
}

/// Parcourt récursivement `dir` et ajoute dans `acc` tous les fichiers dont le
/// nom (insensible à la casse) vaut `nom`.
fn collecter(dir: &Path, nom: &str, acc: &mut Vec<PathBuf>) {
    // `read_dir` peut échouer (dossier illisible) : on ignore silencieusement.
    let Ok(entrees) = fs::read_dir(dir) else { return };
    for e in entrees.flatten() {
        let p = e.path();
        if p.is_dir() {
            collecter(&p, nom, acc); // récursion dans les sous-dossiers
        } else if p
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.eq_ignore_ascii_case(nom))
            .unwrap_or(false)
        {
            acc.push(p);
        }
    }
}

/// Vrai si le chemin passe par un dossier `ombstx` SUIVI (plus bas) d'un
/// dossier `offline`. On compare en minuscules pour être insensible à la casse.
fn sous_ombstx_offline(p: &Path) -> bool {
    let comps: Vec<String> = p
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_lowercase())
        .collect();
    if let Some(i) = comps.iter().position(|c| c == "ombstx") {
        comps[i + 1..].iter().any(|c| c == "offline")
    } else {
        false
    }
}

/// Traite UN fichier SUBBLK.DBF. Renvoie le nombre de protections enlevées.
fn traiter_fichier(path: &Path) -> Result<usize, Box<dyn Error>> {
    let mut doc = Dbf::open(path)?;

    let champ = doc
        .field(COLONNE)
        .ok_or_else(|| format!("colonne « {} » absente", COLONNE))?;

    // Copie de sécurité : <fichier>.bak (l'original tel quel, avant écrasement).
    let mut bak: PathBuf = path.to_path_buf();
    let ext = bak.extension().and_then(|e| e.to_str()).unwrap_or("dbf").to_string();
    bak.set_extension(format!("{}.bak", ext));
    fs::write(&bak, &doc.bytes)?;

    // Tout à 0, puis on écrase le fichier d'entrée.
    let n = doc.clear_numeric_to_zero(&champ);
    doc.save(path)?;
    Ok(n)
}

/// Localise les SUBBLK.DBF d'un projet `.S7P` (sous `ombstx\offline`).
fn trouver_cibles(s7p: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let dossier = s7p
        .parent()
        .ok_or("impossible de déterminer le dossier du projet")?;

    let mut cibles = Vec::new();
    collecter(dossier, CIBLE, &mut cibles);
    // On ne garde que ceux situés sous ombstx\offline.
    cibles.retain(|p| sous_ombstx_offline(p));
    cibles.sort();
    Ok(cibles)
}

/// Traite une liste de fichiers et construit le bilan.
fn traiter_tous(cibles: &[PathBuf]) -> Rapport {
    let mut r = Rapport::default();
    for p in cibles {
        match traiter_fichier(p) {
            Ok(n) => {
                r.fichiers += 1;
                r.protections += n;
                r.details.push(format!("{}  →  {}", p.display(), n));
            }
            Err(e) => r.erreurs.push(format!("{}  →  {}", p.display(), e)),
        }
    }
    r
}

// ───────────────────────────── Mode GRAPHIQUE ─────────────────────────────
#[cfg(feature = "gui")]
fn main() {
    use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};

    // 1) Choisir le fichier projet *.S7P.
    let choix = FileDialog::new()
        .add_filter("Projet Step7 (*.S7P)", &["s7p", "S7P"])
        .set_title("Choisir le fichier projet .S7P")
        .pick_file();
    let s7p = match choix {
        Some(p) => p,
        None => return, // annulé
    };

    // 2) Repérer les SUBBLK.DBF sous ombstx\offline.
    let cibles = match trouver_cibles(&s7p) {
        Ok(c) => c,
        Err(e) => {
            MessageDialog::new()
                .set_level(MessageLevel::Error)
                .set_title("Erreur")
                .set_description(format!("{}", e))
                .show();
            return;
        }
    };

    if cibles.is_empty() {
        MessageDialog::new()
            .set_level(MessageLevel::Warning)
            .set_title("dbf_protection")
            .set_description("Aucun fichier SUBBLK.DBF trouvé sous « ombstx\\offline ».")
            .show();
        return;
    }

    // 3) Confirmation (opération qui écrase les fichiers).
    let reponse = MessageDialog::new()
        .set_level(MessageLevel::Warning)
        .set_title("Confirmer")
        .set_description(format!(
            "{} fichier(s) SUBBLK.DBF trouvé(s) sous ombstx\\offline.\n\n\
             Ils vont être MODIFIÉS SUR PLACE (colonne {} → 0),\n\
             avec une copie .bak de chacun. Continuer ?",
            cibles.len(),
            COLONNE
        ))
        .set_buttons(MessageButtons::OkCancel)
        .show();
    if reponse != MessageDialogResult::Ok {
        return;
    }

    // 4) Traitement + boîte « OK » finale avec le total.
    let r = traiter_tous(&cibles);

    let mut msg = format!(
        "Traitement terminé.\n\n\
         Fichiers SUBBLK.DBF traités : {}\n\
         Protections enlevées (total) : {}",
        r.fichiers, r.protections
    );
    if !r.erreurs.is_empty() {
        msg.push_str(&format!("\n\n⚠ {} fichier(s) en erreur :\n", r.erreurs.len()));
        msg.push_str(&r.erreurs.join("\n"));
    }

    MessageDialog::new()
        .set_level(if r.erreurs.is_empty() { MessageLevel::Info } else { MessageLevel::Warning })
        .set_title("dbf_protection")
        .set_description(msg)
        .set_buttons(MessageButtons::Ok) // la fameuse boîte « OK » façon VB
        .show();
}

// ───────────────────────────── Mode CONSOLE ──────────────────────────────
// Compilé seulement avec `--no-default-features` (tests sans écran).
#[cfg(not(feature = "gui"))]
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage (mode console) : {} <projet.S7P>", args[0]);
        std::process::exit(2);
    }
    let s7p = Path::new(&args[1]);
    let cibles = trouver_cibles(s7p)?;
    println!("{} fichier(s) SUBBLK.DBF sous ombstx\\offline :", cibles.len());
    let r = traiter_tous(&cibles);
    for d in &r.details {
        println!("  {}", d);
    }
    for e in &r.erreurs {
        eprintln!("  ERREUR {}", e);
    }
    println!(
        "\nFichiers traités : {}\nProtections enlevées (total) : {}",
        r.fichiers, r.protections
    );
    Ok(())
}
