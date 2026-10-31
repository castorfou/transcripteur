# Les Plugins s'intègrent à la compilation, sans chargement dynamique

Chaque Plugin (Source, Moteur, Enrichissement, Rendu) est une crate du workspace Cargo, activée par une feature et recensée dans une liste du binaire. Rust n'a pas d'ABI stable : charger des bibliothèques à l'exécution imposerait une frontière en C, fragile et coûteuse, pour un outil qui n'a pas d'auteurs de Plugins tiers. Le mot « Plugin » désigne donc un point d'extension du code, pas un fichier qu'on dépose.

## Considered Options

- **Bibliothèques dynamiques** (`.so`, `.dll`) : extensibles sans recompiler, mais il faut un contrat C et le maintenir sur Linux et Windows.
- **Processus séparés** : isolement fort, mais il faut définir un protocole, pour un seul utilisateur aujourd'hui.

## Consequences

- Ajouter un Plugin demande de recompiler l'outil.
- Tous les Plugins compilent ensemble : changer un type du contrat, par exemple ajouter un champ à l'Énoncé, signale à la compilation tout ce qui doit suivre, sans gestion de version du contrat.
- La configuration d'un Plugin est un struct typé, rempli par l'écran de lancement, passé à sa fabrique au démarrage de la Session et figé ensuite.
- Un Moteur distant reste un Plugin compilé dans l'outil : c'est son client qui s'intègre, le service qu'il appelle vit ailleurs.
