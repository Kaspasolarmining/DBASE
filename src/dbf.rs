//! Module `dbf` : lecture / écriture bas niveau d'un fichier DBF (dBase IV).
//!
//! Ce module ne dépend QUE de la bibliothèque standard : il n'a aucun lien avec
//! l'interface graphique. On peut donc le tester en ligne de commande, sans
//! écran. C'est une bonne habitude en Rust : séparer la logique « métier » de
//! l'interface utilisateur.

use std::error::Error;
use std::fs;
use std::path::Path;

/// Un champ (colonne) décrit dans l'en-tête du DBF.
#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,   // nom de la colonne, ex. "PASSWORD"
    #[allow(dead_code)] // conservé pour la lisibilité / usages futurs
    pub kind: char,     // type dBase : 'C' texte, 'N' numérique, 'M' mémo…
    pub length: usize,  // largeur du champ en octets
    pub offset: usize,  // position du champ DANS un enregistrement
}

/// Un fichier DBF chargé entièrement en mémoire.
pub struct Dbf {
    pub bytes: Vec<u8>,       // le fichier entier, octet par octet
    pub header_len: usize,    // taille de l'en-tête (début des données)
    pub record_len: usize,    // taille d'un enregistrement
    pub record_count: usize,  // nombre d'enregistrements
    pub fields: Vec<Field>,   // les colonnes
}

impl Dbf {
    /// Analyse un tampon d'octets déjà chargé (par ex. lu depuis un fichier).
    pub fn parse(bytes: Vec<u8>) -> Result<Dbf, Box<dyn Error>> {
        if bytes.len() < 32 {
            return Err("fichier trop court pour être un DBF".into());
        }

        // En-tête principal (little-endian) :
        //   octets 4..8  = nombre d'enregistrements (u32)
        //   octets 8..10 = taille de l'en-tête       (u16)
        //   octets 10..12= taille d'un enregistrement (u16)
        let record_count =
            u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
        let header_len = u16::from_le_bytes([bytes[8], bytes[9]]) as usize;
        let record_len = u16::from_le_bytes([bytes[10], bytes[11]]) as usize;

        // Descripteurs de champs : blocs de 32 octets après l'en-tête,
        // jusqu'à l'octet 0x0D qui marque la fin de la liste.
        let mut fields = Vec::new();
        let mut pos = 32;
        let mut offset = 1; // +1 : 1er octet d'un enregistrement = drapeau de suppression
        while pos < bytes.len() && bytes[pos] != 0x0D {
            let raw = &bytes[pos..pos + 11];
            let end = raw.iter().position(|&b| b == 0).unwrap_or(11);
            let name = String::from_utf8_lossy(&raw[..end]).to_string();
            let kind = bytes[pos + 11] as char;
            let length = bytes[pos + 16] as usize;

            fields.push(Field { name, kind, length, offset });
            offset += length;
            pos += 32;
        }

        Ok(Dbf { bytes, header_len, record_len, record_count, fields })
    }

    /// Lit un fichier DBF depuis le disque.
    pub fn open(path: &Path) -> Result<Dbf, Box<dyn Error>> {
        let bytes = fs::read(path)?;
        Dbf::parse(bytes)
    }

    /// Cherche une colonne par son nom (insensible à la casse).
    pub fn field(&self, name: &str) -> Option<Field> {
        self.fields
            .iter()
            .find(|f| f.name.eq_ignore_ascii_case(name))
            .cloned()
    }

    /// Remet à `0` toutes les valeurs d'une colonne numérique
    /// (= « protection : non » partout). Renvoie le nombre d'enregistrements
    /// réellement modifiés.
    pub fn clear_numeric_to_zero(&mut self, field: &Field) -> usize {
        let mut changed = 0usize;
        for i in 0..self.record_count {
            let start = self.header_len + i * self.record_len + field.offset;
            let cell = &self.bytes[start..start + field.length];

            // Déjà à 0 ? (on ignore les espaces de cadrage)
            if String::from_utf8_lossy(cell).trim() == "0" {
                continue;
            }

            // Champ numérique dBase : cadré à DROITE, complété d'espaces.
            // Largeur 11 -> "          0".
            let text = format!("{:>width$}", "0", width = field.length);
            self.bytes[start..start + field.length].copy_from_slice(text.as_bytes());
            changed += 1;
        }
        changed
    }

    /// Écrit le fichier (modifié) sur le disque.
    pub fn save(&self, path: &Path) -> Result<(), Box<dyn Error>> {
        fs::write(path, &self.bytes)?;
        Ok(())
    }
}
