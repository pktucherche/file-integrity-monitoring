# file-integrity-monitoring
Ce projet en Rust surveille les opérations (modifications, créations, suppressions, déplacements, etc.) sur des dossiers et fichiers.

## Utilisation
```
sudo cargo run
```
Le projet utilise une interface web en : localhost:6077 ; celle-ci est divisée en deux parties:
- Mettre le chemin des dossiers à surveiller (en récursif)
- Affiche les opérations : type opération - path vers le fichier concerné - date - et si il y a une modification voir la diff entre la dernière version du fichier et celui-ci.
Enfin pour sauvegarder tout cela le système utilise une base de donnée sql dans database.db.
