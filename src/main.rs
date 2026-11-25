//! dbf_protection — un petit outil en ligne de commande pour « déverrouiller »
//! la colonne de protection d'un fichier DBF (format dBase IV).
//!
//! But concret : dans la table Siemens `SUBBLK`, la colonne `PASSWORD` contient
//! un drapeau de protection : `3` = bloc protégé, `0` = non protégé.
//! On veut remettre tout le monde à `0` (= protection « non » partout).
//!
//! ─────────────────────────────────────────────────────────────────────────
//! NOTES D'APPRENTISSAGE RUST (lis-les, c'est le but du projet) :
//!
//! * `Vec<u8>`      : un tableau d'octets dynamique. On charge le fichier entier
//!                    dedans, on modifie quelques octets, on réécrit le tout.
//! * `&[u8]`        : une « tranche » (slice) en lecture seule sur ces octets.
//! * `Result<T, E>` : type de retour quand une opération peut échouer. On propage
//!                    l'erreur avec l'opérateur `?` au lieu de `try/catch`.
//! * `Box<dyn Error>`: « n'importe quelle erreur » — pratique pour un main simple.
//! * Pas de valeur `null` en Rust : l'absence se modélise avec `Option<T>`.
//! ─────────────────────────────────────────────────────────────────────────

use std::error::Error;
use std::fs;
use std::path::Path;

/// Description d'un champ (colonne) tel qu'il est décrit dans l'en-tête du DBF.
///
/// `#[derive(Debug)]` demande au compilateur de générer de quoi afficher la
/// structure avec `{:?}` (très pratique pour déboguer).
#[derive(Debug)]
struct Field {
    name: String,   // nom de la colonne, ex. "PASSWORD"
    kind: char,     // type dBase : 'C' texte, 'N' numérique, 'M' mémo, ...
    length: usize,  // largeur du champ en octets
    offset: usize,  // position du champ À L'INTÉRIEUR d'un enregistrement
}

/// Un fichier DBF chargé en mémoire, décortiqué juste ce qu'il faut.
struct Dbf {
    bytes: Vec<u8>,        // le fichier entier, octet par octet
    header_len: usize,     // taille de l'en-tête (où commencent les données)
    record_len: usize,     // taille d'un enregistrement
    record_count: usize,   // nombre d'enregistrements annoncé par l'en-tête
    fields: Vec<Field>,    // la liste des colonnes
}

impl Dbf {
    /// Lit un fichier DBF depuis le disque et analyse son en-tête.
    fn open(path: &Path) -> Result<Dbf, Box<dyn Error>> {
        // `fs::read` renvoie Result<Vec<u8>, _>. Le `?` extrait le Vec ou
        // propage l'erreur (fichier introuvable, etc.) vers l'appelant.
        let bytes = fs::read(path)?;

        if bytes.len() < 32 {
            return Err("fichier trop court pour être un DBF".into());
        }

        // --- En-tête principal (32 octets) ---------------------------------
        // Les entiers DBF sont stockés en « petit-boutiste » (little-endian) :
        // l'octet de poids faible en premier.
        //
        // octets 4..8  : nombre d'enregistrements (u32)
        // octets 8..10 : taille de l'en-tête       (u16)
        // octets 10..12: taille d'un enregistrement (u16)
        let record_count = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
        let header_len = u16::from_le_bytes([bytes[8], bytes[9]]) as usize;
        let record_len = u16::from_le_bytes([bytes[10], bytes[11]]) as usize;

        // --- Descripteurs de champs ---------------------------------------
        // Ils suivent l'en-tête principal, par blocs de 32 octets, jusqu'à
        // rencontrer l'octet 0x0D qui marque la fin de la liste des champs.
        let mut fields = Vec::new();
        let mut pos = 32;      // position de lecture dans l'en-tête
        let mut offset = 1;    // +1 : le 1er octet d'un enregistrement est le
                               //      drapeau de suppression (' ' ou '*').
        while pos < bytes.len() && bytes[pos] != 0x0D {
            // Nom : 11 octets, complétés par des zéros (\0).
            let raw = &bytes[pos..pos + 11];
            let end = raw.iter().position(|&b| b == 0).unwrap_or(11);
            let name = String::from_utf8_lossy(&raw[..end]).to_string();

            let kind = bytes[pos + 11] as char; // octet 11 : type du champ
            let length = bytes[pos + 16] as usize; // octet 16 : longueur

            fields.push(Field { name, kind, length, offset });

            offset += length; // le champ suivant commence juste après
            pos += 32;         // descripteur suivant
        }

        Ok(Dbf { bytes, header_len, record_len, record_count, fields })
    }

    /// Cherche une colonne par son nom (insensible à la casse).
    /// Renvoie `Option` : `Some(&Field)` si trouvée, `None` sinon.
    fn field(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.name.eq_ignore_ascii_case(name))
    }

    /// Octets bruts de la colonne `field` dans l'enregistrement n°`i`.
    fn cell(&self, i: usize, field: &Field) -> &[u8] {
        let start = self.header_len + i * self.record_len + field.offset;
        &self.bytes[start..start + field.length]
    }

    /// Écrit une valeur numérique dans une cellule, cadrée à DROITE et
    /// complétée par des espaces à gauche — c'est la convention dBase pour
    /// les champs de type 'N'. Exemple pour largeur 11 : "          0".
    fn set_numeric(&mut self, i: usize, field: &Field, value: &str) {
        let start = self.header_len + i * self.record_len + field.offset;
        let width = field.length;

        // On fabrique la chaîne cadrée à droite. `{:>width$}` = alignement
        // à droite sur `width` caractères.
        let text = format!("{:>width$}", value, width = width);
        let src = text.as_bytes();

        // Recopie octet par octet dans le tampon du fichier.
        // (On fait simple et explicite, c'est pédagogique.)
        for k in 0..width {
            self.bytes[start + k] = src[k];
        }
    }

    /// Sauvegarde le fichier modifié sur le disque.
    fn save(&self, path: &Path) -> Result<(), Box<dyn Error>> {
        fs::write(path, &self.bytes)?;
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // --- Analyse ultra-simple des arguments de la ligne de commande --------
    // Usage : dbf_protection <entrée.dbf> <sortie.dbf> [NOM_COLONNE]
    // La colonne vaut "PASSWORD" par défaut (le drapeau de protection Siemens).
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage : {} <entrée.dbf> <sortie.dbf> [NOM_COLONNE=PASSWORD]", args[0]);
        std::process::exit(2);
    }
    let input = Path::new(&args[1]);
    let output = Path::new(&args[2]);
    let column = args.get(3).map(|s| s.as_str()).unwrap_or("PASSWORD");

    // --- Ouverture et diagnostic --------------------------------------------
    let mut dbf = Dbf::open(input)?;
    println!("Fichier      : {}", input.display());
    println!("Enregistrements : {}", dbf.record_count);
    println!("Colonnes      : {}", dbf.fields.len());

    // On récupère les infos de la colonne cible. On clone les champs utiles
    // (name/kind/length/offset) pour ne pas garder un emprunt sur `dbf` :
    // sinon on ne pourrait pas appeler `set_numeric` (qui modifie `dbf`).
    let target = match dbf.field(column) {
        Some(f) => Field { name: f.name.clone(), kind: f.kind, length: f.length, offset: f.offset },
        None => {
            eprintln!(
                "⚠ Colonne « {} » introuvable dans ce fichier. Colonnes présentes : {}",
                column,
                dbf.fields.iter().map(|f| f.name.clone()).collect::<Vec<_>>().join(", ")
            );
            std::process::exit(1);
        }
    };
    println!("Colonne cible : {} (type {}, largeur {})", target.name, target.kind, target.length);

    // --- Comptage avant modification (pour vérifier le résultat) -----------
    let mut changed = 0usize;
    for i in 0..dbf.record_count {
        // On lit la valeur, on enlève les espaces autour (`trim`).
        let raw = dbf.cell(i, &target);
        let current = String::from_utf8_lossy(raw);
        if current.trim() != "0" {
            changed += 1;
            // On met « 0 » (= protection « non »).
            dbf.set_numeric(i, &target, "0");
        }
    }

    // --- Sauvegarde ---------------------------------------------------------
    dbf.save(output)?;

    println!("→ {} enregistrement(s) remis à 0 (protection = non).", changed);
    println!("→ Fichier modifié écrit : {}", output.display());
    Ok(())
}
