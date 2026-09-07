# Correctifs de dépendances — 8 septembre 2026

Ces modifications corrigent les sources ; les exécutables déjà distribués
ne changent pas tant qu'une nouvelle compilation n'a pas été validée et livrée.

## Bibliothèques maintenues

- Client : openssl 0.10.81 et OpenSSL embarqué 3.6.3, fuser 0.16.0,
  rand 0.8.8 / 0.9.5.
- L'ancien exemple d'enregistrement n'utilise plus quest/rpassword. Sa
  confirmation d'écrasement utilise la bibliothèque standard et refuse
  l'écrasement sur fin de fichier ou erreur de lecture.
- Les lockfiles obsolètes de portable et virtual_display ont été supprimés :
  Cargo utilise le Cargo.lock racine pour ces membres du workspace. Ils
  restent récupérables dans l'historique Git.
- Serveur : l'analyseur de configuration optionnel de deadpool, inutilisé
  par le pool SQLite, est désactivé. Cela retire nom 5 / lexical-core 0.7.
- Interface Windows historique du serveur : Tauri 1.8.3, Vite 6.4.3,
  dépendances Rust réactualisées, package-lock.json suivi, serveur de
  développement et prévisualisation liés à 127.0.0.1. Cette interface
  n'est pas incluse dans l'image hbbs/hbbr déployée sur Oracle.

## Correctifs locaux transparents

`libs/atty-compat` réimplémente la petite API atty avec
`std::io::IsTerminal`, sans aucun code unsafe. Ce n'est pas une nouvelle
version amont d'atty ; son numéro de compatibilité 0.2.14 est conservé.

Le correctif officiel GLib [gtk-rs #1343](https://github.com/gtk-rs/gtk-rs-core/pull/1343)
est repris dans la version compatible GTK3 utilisée par chaque dépôt.
Les sources complètes, licences MIT, provenance et tests sont inclus dans
`vendor/`. Le numéro de version amont n'est pas artificiellement augmenté.
L'interface serveur utilise aussi une adaptation de phf_generator 0.8 qui
remplace rand 0.7 par rand 0.8.6+, testée avec la fonction de recherche PHF.

Un audit limité aux numéros de version peut encore signaler ces composants ;
inversement, cargo-audit ne vérifie pas le code des dépendances locales.
Le script `tools/check_security_dependencies.py` vérifie donc les chemins
effectifs, les empreintes des corrections locales et les versions minimales.
Aucune exception globale RustSec ni clôture manuelle d'alerte n'est ajoutée.

## Vérification

Le workflow `security-dependencies.yml` contrôle :

- les planchers de sécurité et les correctifs locaux ;
- cargo-audit, avec échec sur vulnérabilité ou avertissement « unsound » ;
- atty sur Windows et Linux, notamment avec des flux redirigés ;
- l'itérateur GLib en compilation optimisée (condition du défaut d'origine) ;
- le presse-papiers Linux avec FUSE, dans le dépôt client ;
- la compilation Windows et les tables PHF de l'interface serveur.

Les workflows de compilation complète restent nécessaires avant distribution.
Les tests CI ne remplacent pas la recette réelle d'une session distante,
du transfert de fichiers et du presse-papiers entre deux machines.

Les avertissements de maintenance (GTK3, sodiumoxide, etc.) restent visibles :
ce sont des dettes techniques à suivre, pas des vulnérabilités que ce document
prétend avoir éliminées. Les correctifs de dépendances ne constituent pas
une preuve d'absence de toute faille dans l'application.
