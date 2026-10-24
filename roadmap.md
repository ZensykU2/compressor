# Roadmap: Eigener Kompressor & Audio-Codec mit Context Mixing

**Ziel:** Von Grund auf einen eigenen Kompressor programmieren, der lernende Vorhersagen gewichtet (Context Mixing), und ihn später zu einem verlustfreien Audio-Codec ausbauen. Alles selbst geschrieben, Open Source und messbar.

**Annahmen:**
- ca. **5 Stunden pro Woche** neben der Lehre
- Sprache: **Rust**
- Keine GPU nötig, alles läuft auf einer normalen CPU

> Die Wochenangaben sind Richtwerte. Wenn eine Phase länger dauert, ist das normal, vor allem Phase 2.

---

## Projektziel (nach Recherche auf encode.su und Hydrogenaudio, Oktober 2026)

**Ein verlustfreier Audio-Codec in Rust, der:**
1. **Open Source** ist und eine **schriftliche Format-Spezifikation** hat (nicht nur Code),
2. **nur mit Ganzzahlen** rechnet und deshalb auf jeder Plattform **bitgenau gleich** dekodiert,
3. die Residuen mit **Context Mixing** kodiert (mehrere lernende Modelle, gewichtet gemischt),
4. als eigenes Experiment **getrennte Prädiktoren für tonale und perkussive Anteile** mischt,
5. bei der Kompression zwischen TAK/Monkey's Audio und sac liegt und trotzdem **schneller als Echtzeit dekodiert**.

**Messlatte:** FLAC, SRLA, TAK, Monkey's Audio, WavPack, HALAC und sac auf denselben Testdateien.

### Was es schon gibt (Stand der Recherche)

| Codec | Stärke | Einschränkung |
|---|---|---|
| **FLAC** | Sehr schnell, überall abspielbar, formale Spezifikation (RFC 9639) | Schwächste Kompression unter den guten Codecs |
| **TAK** | Kompression etwa wie Monkey's Audio «High», Dekodieren fast so schnell wie FLAC | Closed Source, nur Windows; Open-Source-Decoder nur durch Reverse Engineering (FFmpeg) |
| **SRLA** | Open Source (MIT), Ziel: schnelles Dekodieren + gute Kompression, nahe an TAK | Encoder sehr langsam; LPC-basiert, kein Context Mixing |
| **HALAC** | Sehr schnell, multi-threaded, ANS statt Rice-Kodierung | Kein Context Mixing, keine Unterstützung in gängigen Playern |
| **Monkey's Audio** | Starke Kompression | Dekodieren kostet deutlich mehr CPU |
| **sac** | Kompression an der Spitze, Context-Mixing-artige Residuenkodierung | Forschungsprojekt, sehr langsam, rechnet mit Gleitkommazahlen |

**Fazit:** «Besser als FLAC und schnell» allein ist **nicht neu** (TAK, SRLA, HALAC). Die Lücke liegt in der **Kombination**: offen, spezifiziert, bitgenau deterministisch, Context Mixing in schneller Form. Für die Trennung in tonale und perkussive Prädiktoren bei verlustfreier Kompression wurde bei der Recherche nichts gefunden, das Experiment ist also möglicherweise neu.

### Weiterführende Links zur Recherche
- SRLA-Thread auf Hydrogenaudio: https://hydrogenaud.io/index.php/topic,126582.0.html
- SRLA Quellcode: https://github.com/aikiriao/SRLA
- HALAC-Thread auf Hydrogenaudio: https://hydrogenaudio.org/index.php/topic,125248.0.html
- HALAC auf GitHub: https://github.com/Hakan-Abbas/HALAC-High-Availability-Lossless-Audio-Compression
- TAK im Hydrogenaudio-Wiki: https://wiki.hydrogenaud.io/index.php?title=TAK
- sac-Thread auf encode.su: https://encode.su/threads/1137-Sac-(State-of-the-Art)-Lossless-Audio-Compression
- Grosser Codec-Vergleich (Martijn van Beurden): https://hydrogenaudio.org/index.php/topic,122508.0.html
- Lesenswert: «Why lossless audio codecs generally suck»: https://codecs.multimedia.cx/2010/11/why-lossless-audio-codecs-generally-suck/

---

## Übersicht

| Phase | Wochen | Thema | Ergebnis |
|---|---|---|---|
| 0 | 1–2 | Setup & Messwerkzeuge | Projekt, Testdaten, Entropie-Rechner |
| 1 | 3–4 | Aufwärmen: RLE & Huffman | Erster funktionierender Kompressor |
| 2 | 5–8 | Arithmetischer Kodierer | Bitgenaues Kodieren mit Wahrscheinlichkeiten |
| 3 | 9–11 | Kontextmodelle | Vorhersage aus den letzten Bytes |
| 4 | 12–15 | Lernender Mixer | **Herzstück: gewichtete Vorhersagen** |
| 5 | 16–19 | Feinschliff & v0.1 | Öffentliches Release auf GitHub |
| 6 | 20–24 | Audio-Grundlagen | FLAC-ähnlicher eigener Codec |
| 7 | 25–32 | Context Mixing für Audio | Eigener lernender Audio-Codec |
| 8 | ab 33 | Vertiefung | Eigene Richtung wählen |

**Meilensteine:**
- **~Woche 15:** Dein Kompressor schlägt ZIP/gzip deutlich.
- **~Woche 19:** Version 0.1 öffentlich auf GitHub.
- **~Woche 32:** Eigener verlustfreier Audio-Codec, verglichen mit FLAC.

---

## Phase 0 – Setup & Messwerkzeuge (Woche 1–2)

**Ziel:** Bevor du komprimierst, musst du messen können.

- [x] Rust-Projekt anlegen (`cargo new`), CLI mit zwei Befehlen: `compress` und `decompress`
- [x] Git-Repo anlegen, von Anfang an öffentlich oder später veröffentlichen
- [x] Testdaten herunterladen: Silesia-Korpus und `enwik8` (100 MB Wikipedia)
- [x] **Entropie-Rechner** schreiben: Wie viele Bits pro Byte braucht eine Datei theoretisch (Order-0)?
- [x] Benchmark-Skript: Grösse, Zeit, Vergleich mit `gzip -9`, `xz -9`, `zstd -19`
- [x] Datei `BENCHMARKS.md` anlegen, in die du jedes Ergebnis einträgst

**Konzepte:** Shannon-Entropie, Bits pro Byte (bpc), Kompressionsrate

**Fertig, wenn:** Du für jede Testdatei die Order-0-Entropie kennst und weisst, was gzip daraus macht.

---

## Phase 1 – Aufwärmen: RLE & Huffman (Woche 3–4)

**Ziel:** Bitweises Lesen/Schreiben beherrschen und einen ersten echten Kompressor haben.

- [ ] **BitWriter / BitReader** schreiben (wirst du überall wiederverwenden)
- [ ] RLE implementieren (beide Konventionen: Anzahl und Anzahl−1)
- [ ] Huffman: Häufigkeiten zählen → Baum bauen → Codes erzeugen → kodieren
- [ ] Huffman-Tabelle in die Datei schreiben, damit der Decoder sie kennt
- [ ] **Roundtrip-Test:** `decompress(compress(x)) == x` für alle Testdateien

**Konzepte:** Präfixfreie Codes, Codebaum, kanonischer Huffman-Code

**Fertig, wenn:** Huffman kommt nahe an die Order-0-Entropie aus Phase 0 heran.

---

## Phase 2 – Arithmetischer Kodierer (Woche 5–8)

**Ziel:** Der Motor, der jede Vorhersage in möglichst wenige Bits verwandelt. Der schwierigste und lehrreichste Teil.

- [ ] Theorie verstehen: Ein Intervall [0,1) wird mit jedem Symbol weiter eingeengt
- [ ] **Binären** arithmetischen Kodierer bauen (kodiert Bit für Bit mit Wahrscheinlichkeit p), wie in lpaq1
- [ ] Ganzzahl-Arithmetik (32 Bit), keine Gleitkommazahlen, damit Encoder und Decoder exakt übereinstimmen
- [ ] Order-0-Modell: zählt pro Bitposition, wie oft 0 oder 1 kam, und passt sich laufend an
- [ ] Viele Roundtrip-Tests, auch mit Spezialfällen: leere Datei, 1 Byte, nur Nullen, Zufallsdaten
- [ ] Optional: **Fuzzing** mit `cargo-fuzz`, findet Randfälle, an die du nie gedacht hättest

**Konzepte:** Intervallteilung, Renormalisierung, adaptive Wahrscheinlichkeiten

**Fertig, wenn:** Der Roundtrip immer klappt und dein Order-0-Kodierer Huffman leicht schlägt.

> **Typische Stolpersteine:** Encoder und Decoder runden unterschiedlich → Datei kaputt. Immer zuerst kleine Testdateien prüfen.

---

## Phase 3 – Kontextmodelle (Woche 9–11)

**Ziel:** Bessere Vorhersage, weil das Modell weiss, was vorher kam.

- [ ] Order-1-Modell: Vorhersage abhängig vom letzten Byte
- [ ] Order-2, -3, -4: Kontext = letzte 2–4 Bytes, gespeichert in einer **Hashtabelle**
- [ ] Zustandszähler / Lernrate: schnell anpassen am Anfang, stabil später
- [ ] Jedes Modell einzeln testen und in `BENCHMARKS.md` eintragen

**Konzepte:** Kontext, Hashing, Kollisionen, «Context Dilution» (zu lange Kontexte haben zu wenig Daten)

**Fertig, wenn:** Du siehst, dass z. B. Order-2 bei Text viel besser ist als Order-0, und verstehst warum.

---

## Phase 4 – Lernender Mixer (Woche 12–15)

**Ziel:** Deine ursprüngliche Idee: Mehrere Vorhersagen werden gewichtet, und die Gewichte lernen während des Kodierens.

- [ ] `stretch(p) = ln(p / (1−p))` und `squash(x) = 1 / (1 + e^−x)` implementieren (als Lookup-Tabelle für Tempo)
- [ ] Mixer: `p_final = squash(Σ gewicht_i · stretch(p_i))`
- [ ] Nach jedem Bit: `gewicht_i += lernrate · fehler · stretch(p_i)` (ein Schritt Gradientenabstieg)
- [ ] Mehrere Gewichtssätze, ausgewählt durch einen kleinen Kontext (z. B. Bitposition)
- [ ] Experimentieren: Lernrate, Anzahl Modelle, Startgewichte

**Konzepte:** Logistische Regression, Online-Lernen, Gradientenabstieg

**Wichtig:** Der Decoder lernt exakt gleich mit. Deshalb muss das Modell nie mitgeschickt werden.

**Fertig, wenn:** Der Mixer besser ist als jedes einzelne Modell. **Meilenstein: deutlich besser als gzip.**

---

## Phase 5 – Feinschliff & Release v0.1 (Woche 16–19)

- [ ] **Match-Modell:** findet lange Wiederholungen und sagt das nächste Byte daraus voraus
- [ ] **APM/SSE:** korrigiert die finale Wahrscheinlichkeit nochmals anhand eines Kontexts
- [ ] Profiling: Wo ist das Programm langsam? (`cargo flamegraph`, Benchmarks mit `criterion`)
- [ ] README schreiben: Was es ist, wie es funktioniert, Benchmark-Tabelle, Vergleich mit gzip/xz/zstd
- [ ] Open-Source-Lizenz wählen (z. B. MIT oder Apache-2.0)
- [ ] **Release v0.1 auf GitHub**, optional auf encode.su vorstellen

**Fertig, wenn:** Jemand anderes dein Projekt klonen, bauen und die Ergebnisse nachprüfen kann.

---

## Phase 6 – Audio-Grundlagen (Woche 20–24)

**Ziel:** Verstehen, warum Audio anders ist als Text, und einen einfachen eigenen Codec bauen.

- [ ] WAV-Dateien lesen/schreiben (Crate `hound` oder selbst den Header parsen)
- [ ] **Lineare Prädiktion:** jedes Sample aus den vorherigen vorhersagen, nur den Fehler (Residuum) speichern
- [ ] Feste Prädiktoren zuerst (wie FLAC: Ordnung 0–4), dann LPC mit berechneten Koeffizienten
- [ ] Residuen mit **Rice-Kodierung** speichern
- [ ] Stereo: Mid/Side-Kodierung (L+R und L−R) ausprobieren
- [ ] Vergleich mit `flac -8` auf denselben Dateien

**Konzepte:** Abtastrate, Bittiefe, Prädiktion, Residuum, Golomb-/Rice-Codes

**Fertig, wenn:** Dein Codec bitgenau dekodiert und in der Nähe von FLAC liegt.

---

## Phase 7 – Context Mixing für Audio (Woche 25–32)

**Ziel:** Deine beiden Bausteine verbinden. Jetzt wird es wirklich dein eigener Codec.

- [ ] Mehrere Prädiktoren parallel: LPC, adaptive Filter (NLMS), Kurz- und Langzeit
- [ ] Prädiktoren mischen (Kaskade: der nächste sagt den Fehler des vorherigen voraus)
- [ ] Residuen nicht mit Rice, sondern mit deinem **arithmetischen Kodierer + Kontextmodellen** kodieren
- [ ] Bitplane-Kodierung der Residuen ausprobieren
- [ ] Benchmark gegen FLAC, SRLA, TAK, HALAC, Monkey's Audio und als Obergrenze gegen sac/OptimFROG
- [ ] **Eigenes Experiment:** Signal in tonale und perkussive Anteile trennen (z. B. Medianfilter), je einen Prädiktor pro Anteil, im Mixer kombinieren. Die Trennung muss im Decoder exakt gleich und nur aus bereits dekodierten Samples berechnet werden.
- [ ] Dekodiergeschwindigkeit messen (Vielfaches von Echtzeit). Ziel: deutlich schneller als Echtzeit
- [ ] Format-Spezifikation als eigenes Dokument schreiben (Vorbild: RFC 9639 von FLAC)

**Fertig, wenn:** Dein Codec FLAC klar schlägt. **Release v0.2.**

---

## Phase 8 – Vertiefung (ab Woche 33)

Wähle eine Richtung, je nachdem, was dich am meisten packt:

- **Text-Kompression:** an `enwik8` arbeiten und dich mit dem Large Text Compression Benchmark messen. Der Hutter Prize (enwik9) ist der Gipfel, dort liegt der Rekord bei rund 110 MB.
- **Verlustbehaftetes Audio:** MDCT, Quantisierung, Psychoakustik. Guter Einstieg: das QOA-Format verstehen und nachbauen.
- **Bilder:** QOI nachbauen, dann einen eigenen verlustfreien Bildcodec mit Context Mixing.
- **Video:** Bewegungsschätzung, Block-Prädiktion. In Rust kannst du dir rav1e (AV1-Encoder) ansehen.
- **Schreiben:** Einen Blogpost oder kurzen technischen Bericht über deinen Codec veröffentlichen.

---

## Ressourcen

### Grundlagen & Theorie
- **Data Compression Explained** – Matt Mahoney (kostenlos, die wichtigste Quelle für dein Projekt): https://mattmahoney.net/dc/dce.html
- **C. E. Shannon – «A Mathematical Theory of Communication» (1948):** das Ursprungspaper der Informationstheorie
- **Witten, Neal, Cleary – «Arithmetic Coding for Data Compression» (1987):** der Klassiker zum arithmetischen Kodieren
- Bücher: *Introduction to Data Compression* (Khalid Sayood), *Data Compression: The Complete Reference* (David Salomon), *Understanding Compression* (Colt McAnlis, Aleks Haecky, sehr einsteigerfreundlich)

### Context Mixing & Referenz-Code
- Matt Mahoneys Seite mit PAQ, **lpaq1** (kurz und gut lesbar) und vielem mehr: https://mattmahoney.net/dc/
- **paq8px** (aktive Weiterentwicklung von PAQ): https://github.com/hxim/paq8px
- **cmix** (aktuelle Spitze, sehr gross): https://github.com/byronknoll/cmix

### Entropiekodierung
- Blog von Fabian Giesen (ANS, rANS, viele Tiefenartikel): https://fgiesen.wordpress.com/
- ryg_rans (kompakte rANS-Implementierung): https://github.com/rygorous/ryg_rans

### Audio
- **FLAC-Spezifikation (RFC 9639):** https://www.rfc-editor.org/rfc/rfc9639.html
- **sac** (verlustfreier Audio-Codec mit Context-Mixing-Ideen, gutes Vorbild für Phase 7): https://github.com/slmdev/sac
- **QOA** (einfaches verlustbehaftetes Audioformat, ca. 400 Zeilen C): https://qoaformat.org
- **Opus-Spezifikation (RFC 6716):** https://www.rfc-editor.org/rfc/rfc6716
- **Hydrogenaudio Wiki** (Codec-Vergleiche, Hörtests): https://wiki.hydrogenaud.io/
- **Xiph.org Videos** «A Digital Media Primer for Geeks» (Monty, hervorragende Einführung in digitales Audio/Video): https://xiph.org/video/

### Bilder & Video (für später)
- **QOI** (einfaches verlustfreies Bildformat): https://qoiformat.org
- **rav1e** (AV1-Encoder in Rust): https://github.com/xiph/rav1e

### Testdaten & Benchmarks
- **Silesia-Korpus:** https://sun.aei.polsl.pl/~sdeor/index.php?page=silesia
- Übersicht weiterer Korpora (Calgary, Canterbury …): https://www.data-compression.info/Corpora/
- **enwik8 / enwik9:** https://mattmahoney.net/dc/textdata.html
- **Large Text Compression Benchmark:** https://mattmahoney.net/dc/text.html
- **Hutter Prize:** http://prize.hutter1.net/
- Für Audio: eigene Aufnahmen, CC-lizenzierte Musik oder die EBU-SQAM-Testaufnahmen. Wichtig: verschiedene Genres und Sprache testen.

### Community
- **encode.su:** das Forum der Kompressions-Community, dort sind auch die Autoren von PAQ, cmix und sac aktiv: https://encode.su/

### Rust-Werkzeuge
- The Rust Book: https://doc.rust-lang.org/book/
- The Rust Performance Book: https://nnethercote.github.io/perf-book/
- `criterion` (Benchmarks): https://github.com/bheisler/criterion.rs
- `cargo-fuzz` (Fuzzing): https://github.com/rust-fuzz/cargo-fuzz
- `hound` (WAV lesen/schreiben): https://github.com/ruuda/hound

---

## Arbeitsweise-Tipps

1. **Roundtrip-Test zuerst.** Jede Änderung muss `decompress(compress(x)) == x` bestehen. Ein Kompressor, der Daten kaputt macht, ist wertlos.
2. **Jede Idee messen.** Trag jedes Experiment in `BENCHMARKS.md` ein, auch die gescheiterten. Daraus lernst du am meisten.
3. **Eine Änderung pro Experiment.** Sonst weisst du nicht, was geholfen hat.
4. **Fremden Code lesen, aber selbst schreiben.** Wenn du feststeckst, schau in lpaq1 oder sac, verstehe das Prinzip, und schreib es dann in deinen eigenen Worten.
5. **Lernjournal führen.** Kurze Notizen, was du verstanden hast. Daraus wird später leicht ein Blogpost oder README.
