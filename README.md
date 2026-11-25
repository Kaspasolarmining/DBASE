# dbf_protection

Petit outil **Rust** pour retirer la protection des blocs Siemens stockés dans
un fichier **DBF (dBase IV)** : il remet à `0` toute la colonne **`PASSWORD`**
(`3` = protégé → `0` = non protégé) de la table `SUBBLK`.

## Interface graphique (Windows)

Double-cliquer sur **`dbf_protection.exe`** :

1. un **explorateur de fichiers** s'ouvre (filtré sur les `.dbf`, nom par
   défaut `SUBBLK.DBF`) — on choisit le fichier à traiter ;
2. une fenêtre de **confirmation** prévient que le fichier va être modifié ;
3. le fichier d'entrée est **écrasé sur place** par la version traitée ;
4. une **copie de sécurité** `*.bak` de l'original est créée à côté, au cas où ;
5. un message indique le nombre de protections remises à `0`.

> La colonne traitée est **toujours `PASSWORD`** (figée).
> L'éventuel fichier mémo associé (`.DBT`) n'est pas touché.

L'exécutable prêt à l'emploi est dans **`dist/dbf_protection.exe`**.

## Compiler soi-même

### Version Windows (depuis Windows)
```bash
cargo build --release
# -> target/release/dbf_protection.exe
```

### Cross-compilation Windows depuis Linux
```bash
sudo apt-get install -y mingw-w64
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
# -> target/x86_64-pc-windows-gnu/release/dbf_protection.exe
```
(Le linker mingw est déjà configuré dans `.cargo/config.toml`.)

### Mode console (tests, sans écran)
```bash
cargo run --no-default-features -- chemin/vers/FICHIER.DBF
```

## Organisation du code (pour apprendre Rust)

- `src/dbf.rs` — lecture / écriture **bas niveau** du format DBF, sans aucune
  dépendance externe (uniquement la bibliothèque standard). Commenté en français.
- `src/main.rs` — l'interface : sélecteur de fichier et boîtes de dialogue
  natives via la bibliothèque [`rfd`](https://crates.io/crates/rfd).

Deux « features » Cargo :
- `gui` (par défaut) : interface graphique ;
- sans `gui` (`--no-default-features`) : mode console.
