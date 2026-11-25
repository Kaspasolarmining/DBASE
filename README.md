# dbf_protection

Outil en ligne de commande (Rust, sans dépendance) pour remettre à `0` la
colonne de protection d'un fichier DBF (dBase IV) — typiquement la colonne
`PASSWORD` des tables Siemens `SUBBLK` (`3` = protégé → `0` = non protégé).

## Usage
```bash
cargo run --release -- <entrée.dbf> <sortie.dbf> [NOM_COLONNE=PASSWORD]
```

Le code (`src/main.rs`) est commenté en français à but pédagogique.
