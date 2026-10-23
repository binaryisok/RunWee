# RunWee — Storytelling, Poetry & Grammar roadmap (phases T0–T6)

Goal: let a teacher make clean, warm, *storybook-quality* animations for Class 6–9
English: short stories, poems and grammar. Everything stays deterministic (a pure
function of time), scriptable from Rhai, and renders on the CPU like the rest of RunWee.

## 1. Research: what the NCERT Class 6–9 English books contain

We used the official NCERT textbook index (ncert.nic.in/textbook.php), the NCERT
prelims/contents pages and CIET audio-book listings **for ideas and structure only**.
**No NCERT text or PDF is in this repository.** (The box's network could not
download ncert.nic.in PDFs directly because the TLS handshake was reset, so contents
pages were read through the public web index.)

| Class | Book (code) | What's in it, and what it suggests |
|---|---|---|
| 6 | **Poorvi** (fepr1, 2024) | Five themed units with three pieces each: Fables & Folk Tales (*A Bottle of Dew*, *The Raven and the Fox* as a verse fable, *Rama to the Rescue* as a **graphic story**), Friendship, Nurturing Nature (*Neem Baba* is a **conversation with a tree**; *What a Bird Thought*), Sports & Wellness, Culture & Tradition. Exercises: "Let us discuss" in-text checks and "Let us think and reflect". This suggests **comic panels**, **talking animals and trees**, **moral cards** and **discussion cards**. |
| 6 | Honeysuckle / A Pact with the Sun (fehl1 / fepw1, older) | Prose paired with short poems (*A House, A Home*, *The Kite*, *The Quarrel*, *Beauty*, *Where Do All the Teachers Go?*, *The Wonderful Words*, *Vocation*). This suggests a **poem-after-story** lesson rhythm. |
| 7 | Honeycomb / An Alien Hand (gehc1 / geah1) | Poems such as *The Squirrel*, *The Rebel*, *The Shed*, *Chivvy*, *Trees*, *Mystery of the Talking Fan*, *Dad and the Cat and the Tree*. These suggest **personification**, **repetition** and **humour** callouts. |
| 8 | **Honeydew** (hehd1) | 8 units. Story units: *The Best Christmas Present in the World* (built around a **letter**), *The Tsunami*, *Glimpses of the Past* (picture panels with captions), *Bepin Choudhury's Lapse of Memory*, *This is Jody's Fawn*, *A Short Monsoon Diary* (**dated diary entries**). Poems: *The Ant and the Cricket*, *Geography Lesson*, *The Last Bargain* (Tagore), *The School Boy* (Blake), *On the Grasshopper and Cricket* (Keats). Exercises: Comprehension Check, Working with the Text, Working with Language, Speaking and Writing. |
| 8 | It So Happened (heih1) | *How the Camel Got His Hump*, *The Selfish Giant*, *Princess September*, *Jalebis*, … These suggest **fable/fairy-tale staging** and **seasons/day-night** changes (*The Selfish Giant*'s winter garden). |
| 9 | **Beehive** (iebe1) | Prose and poem pairs: *The Fun They Had* / *The Road Not Taken*; *The Sound of Music* / *Wind*; *The Little Girl* / *Rain on the Roof*; *A Truly Beautiful Mind* / *The Lake Isle of Innisfree*; *The Snake and the Mirror* / *A Legend of the Northland*; *My Childhood* / *No Men Are Foreign*; *Reach for the Top* / *On Killing a Tree*; *Kathmandu* / *A Slumber Did My Spirit Seal*; *If I Were You* (a **play**). |
| 9 | Moments (iemo1) and Words and Expressions (workbook) | Supplementary stories such as *The Lost Child*, *Iswaran the Storyteller*, *In the Kingdom of Fools*, *The Happy Prince* and *The Last Leaf*. The workbook and the CBSE grammar syllabus cover **tenses, modals, subject–verb concord, determiners, reported speech** (commands, requests, statements, questions) and gap-filling, editing and transformation exercises. |

Recurring exercise and content types that we turn into reusable visuals:

* **Stories**: characters and dialogue, scene changes, letters, diaries, plays, graphic stories,
  a moral, beginning/middle/end and character traits.
* **Poems**: stanzas, rhyme schemes (ABAB, AABB), refrains, meter, imagery, personification,
  similes and metaphors, alliteration, and new words.
* **Questions**: in-text "discuss" prompts, end-of-text reflection, MCQ, gap filling and true/false.
* **Grammar**:
  * parts of speech and determiners/articles;
  * the 12 tenses;
  * active↔passive and direct↔reported speech;
  * subject–verb agreement;
  * prepositions of place;
  * punctuation, clauses and modals.

### Public-domain texts used in examples

Examples use only original text or public-domain poems:

* Robert Frost, *The Road Not Taken* (1916);
* Coates Kinney, *Rain on the Roof* (1849);
* W. B. Yeats, *The Lake Isle of Innisfree* (1890);
* William Blake, *The School Boy* (1789);
* John Keats, *On the Grasshopper and Cricket* (1816);
* Wordsworth, *A Slumber Did My Spirit Seal* (1800) and *Daffodils* (1807);
* Christina Rossetti, *Who Has Seen the Wind?* (1872);
* R. L. Stevenson, *The Wind* from *A Child's Garden of Verses* (1885);
* the folk tale of the thirsty crow, retold in our own words;
* Aesop's fable of the crow and the fox, retold in our own words.

## 2. Research: animation styles to support

| Style | Reference idea | How we render it |
|---|---|---|
| Paper cut-out | Lotte Reiniger / *Charlie and Lola*, layered card | Flat shapes, slightly wobbly edges, soft **drop shadows** that lift with depth, paper-grain image |
| Watercolour / ink wash | Picture-book wash, sumi-e | Noise-modulated pigment field with **edge darkening** (pigment pooling), granulation, wet bloom; tapered **brush strokes** whose width follows pressure noise |
| Shadow puppets | Tholu bommalata, Wayang, Reiniger | Silhouettes with **cut-out lace holes** and rods, on a **warm back-lit screen** with lamp flicker and vignette |
| Flat vector | Kurzgesagt | Limited palettes, rounded geometry, **long shadows**, rim highlights |
| Folk-art borders | Madhubani, Warli, Gond | Double-line borders with fish/leaf/petal repeats (Madhubani); white stick-figure **Warli dancers** (two triangles plus a circle) on terracotta; Gond **dot-and-dash fills** |
| Page turns | Pop-up books | Vertical-fold page curl with back-of-page shading, gutter shadow, stacked page edges |
| Comic panels | Graphic stories (*Rama to the Rescue*) | Panel grid, gutters, **speech, thought, shout and whisper bubbles** with tails, caption boxes |
| Text | Typewriter and handwriting | Glyph-by-glyph typing with caret; **pen-trace** of each glyph outline, then ink fill (handwriting font) |
| Ambience | Monsoon, autumn, night | Rain streaks and splashes, tumbling leaves, fireflies, stars, day↔night sky |

## 3. What we build

| Phase | Module | Contents |
|---|---|---|
| **T0** | fonts, docs | Bundle three OFL fonts (Crimson Text book serif, Patrick Hand handwriting, Courier Prime typewriter) with aliases `story`, `hand` and `courier`; write this roadmap. |
| **T1** | `story` | Warm **palettes**; a rigged vector **character** (skeletal limbs with poses, walk cycle, blinking, lip-flap, facing, mood eyebrows) plus animal critters (bird, fox); **backdrops and props** (layered hills, sun/moon, trees, house, clouds); **speech/thought/shout/whisper bubbles**; timed **narration captions** with typewriter reveal; **transitions** (page turn, iris, ink bleed, paper fold); the **storybook page** (open book, paper grain, drop cap, page numbers); **comic panels**. |
| **T2** | `storyart` | Paper-cutout styling, watercolour washes, ink brush strokes and ink-wash mountains, shadow-puppet screen and silhouettes, flat-vector long shadows, Madhubani/Warli/Gond borders and motifs, rain/leaves/fireflies/stars, and a day–night sky. |
| **T3** | `poetry` | Syllable counter, rhyme keys and automatic **rhyme-scheme** labels (ABAB colours), stanza layout, **typewriter and handwriting line reveals**, meter marks (˘ /), **figure-of-speech callouts**, flip-card **vocabulary cards**. |
| **T4** | `grammar` | Colour-coded **parts of speech** (lexicon tagger plus `word/TAG` overrides), bracket **parse trees** with growth animation, **tense timelines** for all 12 tenses, **sentence morphs** (active↔passive, direct↔reported, with words flying to their new slots), subject–verb agreement links, **preposition scenes** (in/on/under/…), `a`/`an` chooser, punctuation drop-ins, **quiz and fill-in-the-blank cards** with reveal. |
| **T5** | `lesson` | Comprehension question cards, **story map** (beginning/middle/end), **character web**, summary and **moral** scroll, plus our own extras: a plot mountain (Freytag) with a walking marker, a mood meter (emotion graph through the story), a diary/letter page and a word-ladder/word-cloud of new words. |
| **T6** | Rhai | `story.rs` bindings and `examples/scripts/story.rhai`, `poem.rhai` and `grammar.rhai`, so teachers can script lessons; README and AGENTS updates. |

Every phase ships unit tests (for example, syllable and rhyme heuristics, tagger, tree
parser, layout invariants) and an example rendered and visually checked before commit.

## 4. Non-goals and limits

* No speech recognition or automatic lip-sync from audio. Lip-flap is driven by a
  "talking" envelope, which can come from caption timings.
* The POS tagger and rhyme detector are small heuristics meant for classroom
  sentences, not NLP. Teachers can override tags (`run/V`) and rhyme letters.
* Folk-art motifs are respectful stylisations, not reproductions of specific artworks.
