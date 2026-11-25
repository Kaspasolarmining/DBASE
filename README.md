# dbf_protection

Outil **Rust** pour retirer la protection des blocs d'un **projet Siemens Step7**.
Il remet à `0` la colonne **`PASSWORD`** (`3` = protégé → `0` = non protégé) de
**tous** les fichiers `SUBBLK.DBF` du projet.

## Fonctionnement (interface graphique, Windows)

Double-cliquer sur **`dbf_protection.exe`** :

1. un **explorateur de fichiers** s'ouvre, filtré sur les **`*.S7P`** → on choisit
   le fichier projet Step7 ;
2. le programme part du dossier du projet et descend dans **`ombstx\offline`** ;
3. il traite **tous les `SUBBLK.DBF`** trouvés récursivement là-dessous :
   colonne `PASSWORD` remise à `0`, fichier **écrasé sur place**, **copie `.bak`**
   de chaque original créée à côté ;
4. une boîte de dialogue **« OK »** affiche le **nombre total de protections
   enlevées** (et le nombre de fichiers traités).

> Colonne traitée : toujours **`PASSWORD`**. Fichiers mémo `.DBT` non touchés.
> Les `SUBBLK.DBF` situés hors de `ombstx\offline` sont ignorés.

L'exécutable prêt à l'emploi est dans **`dist/dbf_protection.exe`**.

## Compiler soi-même

### Depuis Windows
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
(Linker mingw déjà configuré dans `.cargo/config.toml`.)

### Mode console (tests, sans écran)
```bash
cargo run --no-default-features -- chemin/vers/Projet.S7P
```

## Organisation du code (pour apprendre Rust)

- `src/dbf.rs` — lecture / écriture **bas niveau** du format DBF, sans dépendance
  externe (bibliothèque standard uniquement). Commenté en français.
- `src/main.rs` — l'interface : sélection du `.S7P`, parcours **récursif** de
  `ombstx\offline`, traitement de chaque `SUBBLK.DBF`, puis boîte de dialogue
  finale. Dialogues natifs via [`rfd`](https://crates.io/crates/rfd).

Deux « features » Cargo : `gui` (par défaut, interface graphique) et le mode
console (`--no-default-features`).
