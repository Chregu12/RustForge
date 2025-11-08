# 🚀 RustForge Template Publishing Guide

So veröffentlichst du das RustForge-Starter Template auf GitHub.

---

## 📦 Schritt 1: Template Repository auf GitHub erstellen

### Via GitHub Web Interface

1. **Gehe zu GitHub:** https://github.com/new

2. **Repository erstellen:**
   - Repository name: `RustForge-Starter`
   - Description: `🚀 Laravel-like Rust Framework Starter Template`
   - Public
   - **NICHT** "Initialize with README" (haben wir schon)

3. **Klicke "Create repository"**

---

## 📤 Schritt 2: Template pushen

```bash
cd /Users/christian/Developer/Github_Projekte/RustForge-Starter

# Remote hinzufügen
git remote add origin https://github.com/Chregu12/RustForge-Starter.git

# Push
git branch -M main
git push -u origin main
```

---

## ⚙️ Schritt 3: Als Template Repository markieren

1. **Gehe zu Repository Settings:**
   - https://github.com/Chregu12/RustForge-Starter/settings

2. **Aktiviere "Template repository":**
   - Checkbox bei "Template repository" aktivieren
   - Save

3. **Fertig!** ✅

Jetzt können User auf den grünen **"Use this template"** Button klicken!

---

## 🔧 Schritt 4: Install Script im Hauptframework veröffentlichen

```bash
cd /Users/christian/Developer/Github_Projekte/Rust_DX-Framework

# install.sh commiten
git add install.sh TEMPLATE_PUBLISHING.md
git commit -m "feat: Add Laravel-style installer script

- One-liner installation: bash <(curl -s URL) my-project
- Clones template automatically
- Sets up .env and git
- Beautiful CLI output

Users can now install with:
  bash <(curl -s https://raw.githubusercontent.com/Chregu12/RustForge/main/install.sh) my-project
"

git push origin main
```

---

## ✨ Schritt 5: README im Hauptframework aktualisieren

Füge diese Sektion zum Haupt-README.md hinzu:

```markdown
## 🚀 Quick Start

### Option 1: One-Liner (Empfohlen)

\`\`\`bash
bash <(curl -s https://raw.githubusercontent.com/Chregu12/RustForge/main/install.sh) my-project
cd my-project
cargo run
\`\`\`

### Option 2: GitHub Template

1. Go to https://github.com/Chregu12/RustForge-Starter
2. Click "Use this template"
3. Clone your new repository
4. Run `cargo build && cargo run`

### Option 3: Manual Clone

\`\`\`bash
git clone https://github.com/Chregu12/RustForge-Starter.git my-project
cd my-project
rm -rf .git && git init
cp .env.example .env
cargo run
\`\`\`
```

---

## 🎯 Das Ergebnis

### Für User sieht es SO aus:

```bash
# Terminal Command:
bash <(curl -s https://raw.githubusercontent.com/Chregu12/RustForge/main/install.sh) my-awesome-app

# Output:
╔═══════════════════════════════════════════════════╗
║                                                   ║
║         RustForge Framework Installer             ║
║         Laravel-like Rust Framework               ║
║                                                   ║
╚═══════════════════════════════════════════════════╝

📦 Creating new RustForge project: my-awesome-app

→ Cloning template...
→ Initializing git repository...
→ Setting up environment...

✅ Project created successfully!

╔═══════════════════════════════════════════════════╗
║  Next Steps:                                      ║
╚═══════════════════════════════════════════════════╝

  1. cd my-awesome-app
  2. cargo build
  3. cargo run

Happy coding! 🚀
```

**Perfekt wie Laravel!** 🎉

---

## 📊 Vergleich

| Framework | Installation Command |
|-----------|---------------------|
| **Laravel** | `laravel new my-project` |
| **RustForge** | `bash <(curl -s ...) my-project` |

**Gleiche DX, nur für Rust!** ✨

---

## 🔄 Updates veröffentlichen

Wenn du das Template aktualisierst:

```bash
# In RustForge-Starter
cd /Users/christian/Developer/Github_Projekte/RustForge-Starter

# Änderungen machen
# ...

# Commit & Push
git add .
git commit -m "Update: Better example code"
git push origin main
```

**Alle neuen User bekommen automatisch die neue Version!** 🚀

---

## ✅ Checklist

- [ ] Repository RustForge-Starter auf GitHub erstellt
- [ ] Template gepusht
- [ ] Als "Template repository" markiert
- [ ] install.sh im Hauptframework committed
- [ ] README.md im Hauptframework aktualisiert mit Quick Start
- [ ] Getestet: `bash install.sh test-project`

---

## 🎬 Demo Test

Teste die Installation:

```bash
# In einem tmp Ordner
cd /tmp

# Script direkt testen (vor GitHub push)
bash /Users/christian/Developer/Github_Projekte/Rust_DX-Framework/install.sh test-app

# Sollte funktionieren:
cd test-app
cargo build
cargo run

# Cleanup
cd ..
rm -rf test-app
```

---

**Bereit für Deployment!** 🚀
