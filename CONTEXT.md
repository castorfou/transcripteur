# Transcripteur

Outil de transcription multi-langue : il capte de l'audio, le transcrit dans la langue parlée et le restitue sous différentes formes, à l'écran ou dans des fichiers.

## Language

### Chaîne de traitement

**Source** :
Ce d'où provient l'audio à transcrire, toujours en temps réel : micro ou Son système. L'outil ne lit jamais de fichier lui-même ; pour transcrire une vidéo, l'utilisateur la joue et l'outil capte le Son système.
_Avoid_ : entrée, input, plugin d'entrée

**Son système** :
Source qui capte l'audio émis par l'appareil vers ses haut-parleurs (visio, vidéo, stream), quelle que soit l'application qui le joue.
_Avoid_ : sortie audio, loopback

**Moteur** :
Ce qui convertit l'audio en texte dans la langue parlée.
_Avoid_ : modèle, ASR, transcripteur

**Enrichissement** :
Traitement qui ajoute de l'information à la transcription sans la produire : traduction, détection d'interlocuteurs.
_Avoid_ : plugin de traitement, post-traitement

**Rendu** :
Forme sous laquelle la transcription est restituée : affichage live, texte brut, sous-titres.
_Avoid_ : sortie, export, plugin de sortie

**Plugin** :
Mécanisme d'extension par lequel une Source, un Moteur, un Enrichissement ou un Rendu s'ajoute à l'outil. Ne désigne pas une catégorie.

**Session** :
Une écoute, du lancement à l'arrêt. La Source, la Langue parlée (détectée ou forcée) et les Enrichissements et Rendus actifs sont choisis au lancement et restent les mêmes pendant toute la Session ; l'affichage live est toujours actif.
_Avoid_ : enregistrement, run

### Contenu

**Transcription** :
Le texte produit par le Moteur, dans la langue effectivement parlée.
_Avoid_ : traduction (qui est un Enrichissement distinct)

**Langue parlée** :
La langue détectée (ou forcée) pour un passage d'audio ; elle peut changer au cours d'une même écoute.

**Texte provisoire** :
Transcription affichée au plus vite pendant que l'énoncé est encore prononcé ; elle peut encore changer.
_Avoid_ : partiel, brouillon

**Texte définitif** :
Transcription d'un énoncé terminé, qui ne change plus ; c'est elle que reçoivent le texte brut et les sous-titres.
_Avoid_ : validé, final

### Évaluation

**Corpus d'essai** :
Ensemble d'extraits audio, avec leur transcription de référence quand elle existe, qui sert à juger la qualité et la latence de l'outil.
_Avoid_ : dataset, jeu de test

**Banc d'essai** :
Outil de développement qui fournit l'audio du Corpus d'essai directement au Moteur, au rythme du temps réel, pour mesurer qualité et latence de façon reproductible. Ce n'est pas une Source et il ne fait pas partie de l'outil livré.
_Avoid_ : Source fichier, lecteur de fichier
