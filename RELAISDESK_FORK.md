# Fork serveur RelaisDesk

Ce dépôt dérive de RustDesk Server Community (`rustdesk-server`), révision
amont `a7736be5e40f85bfc141120dce587e836e5d4b80` (branche 1.1.17 au moment du
fork). Il reste distribué sous GNU AGPL v3, comme indiqué par `LICENSE`.

## Changements fonctionnels

- vérification locale de jetons Ed25519 émis par l’API RelaisDesk ;
- preuve de possession Ed25519 pour chaque message sensible ;
- protection anti-rejeu par nonce et fenêtre temporelle courte ;
- séparation des locataires et des rôles `technician` / `viewer` ;
- limite de postes techniciens actifs par licence ;
- refus des relais qui ne réunissent pas un technicien et un viewer du même
  locataire ;
- canal de contrôle périodique permettant au client de couper une session dès
  que le renouvellement d’autorisation échoue.

Le trafic d’écran reste direct quand le NAT le permet. `hbbr` demeure le relais
RustDesk Community de secours ; aucun proxy SOCKS/HTTP applicatif n’est ajouté.

## Configuration obligatoire en production

```text
RELAISDESK_AUTH_REQUIRED=Y
RELAISDESK_AUTH_PUBLIC_KEYS=relaisdesk-1=<cle-publique-base64url>
```

Plusieurs clés peuvent coexister pendant une rotation :

```text
RELAISDESK_AUTH_PUBLIC_KEYS=relaisdesk-1=<ancienne>,relaisdesk-2=<nouvelle>
```

`hbbs` et `hbbr` refusent de démarrer si le contrôle est déclaré obligatoire
sans clé valide. Le format et la procédure de rotation sont documentés dans
`../docs/FORK_AUTHORIZATION.md`.

## Source et redistribution

Toute distribution d’un binaire modifié ou toute mise à disposition du service
sur un réseau doit être accompagnée de l’accès au code source correspondant,
aux scripts de construction et aux modifications, conformément à l’AGPL v3.
Conserver ce fichier, `LICENSE`, le sous-module `libs/hbb_common` modifié et le
commit exact utilisé pour chaque build publié.
