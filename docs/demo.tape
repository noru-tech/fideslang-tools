# VHS script for the README demo: https://github.com/charmbracelet/vhs
# Render with `vhs docs/demo.tape` from the repository root, with `fl` on PATH.
# Every command runs offline with no configuration.

Output docs/demo.gif

Require fl

Set Shell "bash"
Set FontSize 16
Set Width 1200
Set Height 720
Set Padding 20
Set TypingSpeed 40ms
Set Theme "Catppuccin Mocha"

Hide
Type "cd \"$(mktemp -d)\" && clear"
Enter
Show

Type "fl taxonomy tree categories --depth 2 | head -15"
Enter
Sleep 3s

Type "fl taxonomy show user.contact.email"
Enter
Sleep 2s

Type "fl taxonomy search cookie --keys-only"
Enter
Sleep 2s

Hide
Type "clear"
Enter
Type `printf 'dataset:\n  - fides_key: users_db\n    collections:\n      - name: users\n        fields:\n          - name: email\n            data_categories: [user.contact.email]\n          - name: tracking_id\n            data_categories: [user.cookie_id]\nsystem:\n  - fides_key: newsletter\n    system_type: Service\n    ingress:\n      - fides_key: users_db\n        type: dataset\n    privacy_declarations:\n      - data_use: marketing.communications.email\n        data_categories: [user.contact.email]\n        data_subjects: [customer]\n' > datamap.yml`
Enter
Show

Type "cat datamap.yml"
Enter
Sleep 2s

Type "fl validate datamap.yml"
Enter
Sleep 3s

Type "fl cat datamap.yml --format tree"
Enter
Sleep 3s

Type "fl graph datamap.yml --format mermaid | tail -5"
Enter
Sleep 3s
