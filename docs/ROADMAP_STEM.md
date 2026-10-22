# RunWee STEM roadmap

RunWee is a STEM animation framework already, so this layer starts from an audit
of what exists and only adds what is missing.

## Audit: what RunWee already has

| Area | Already present (module) |
|---|---|
| Coordinate systems | `NumberLine`, `Axes` (log axes, labels), `number_plane`, `complex_plane`, `polar_plane`, 3D `axes3d` (`graph`, `three_d`) |
| Plots | function / parametric / polar / implicit plots, scatter, line graphs, area under curve, Riemann rectangles, tangent and secant lines, bar and pie charts, tables, matrices (`graph`) |
| Fields | `vector_field`, stream lines and animated stream lines, time-varying fields (`graph`); point charges, E-field arrows, field lines, equipotentials, wire B-field, bar magnet (`physics`) |
| Transforms | `apply_matrix`, `apply_complex_function`, homotopy, phase flow (`anim`, `extra`) |
| Fractals | Mandelbrot, Julia, Newton fractals (`extra`) |
| Equations | TeX subset with fractions, matrices, cases, aligned; `TransformMatchingTex` term morphing; `Problem` step-by-step solutions (`tex`, `solution`) |
| Mechanics | RK4 trajectories: pendulum, double pendulum, spring-mass, driven oscillator, projectiles with drag, N-body, ideal gas (`sim`); FBDs, inclines, Atwood machines, pulleys, energy bars (`physics`) |
| Waves & optics | travelling / standing waves, wavefronts; thin-lens ray diagrams, curved mirrors, refraction, prism dispersion (`physics`) |
| Circuits | resistor, capacitor, inductor, battery, AC source, bulb, switch, meters, animated current dots (`physics`) |
| Chemistry | periodic table, Bohr atoms, orbital boxes, Lewis dots, orbital shapes, molar mass, equation balancer, 2D/3D molecules from XYZ, lattices, glassware, pH scale (`chemistry`) |
| Biology | cells and organelles, DNA helix, RNA, neuron, mitosis, viruses, Punnett squares, logistic / exponential / predator-prey models (`biology`) |
| Graphs (CS) | graph layouts (circular, spring, tree, grid, shell), `network` drawing (`graph`) |

## What we studied and what is missing

| Tool | Signature visuals | Gap in RunWee → what we build |
|---|---|---|
| **3Blue1Brown / Manim** | Linear-transform grids with basis vectors and eigenvectors, Fourier epicycles drawing any shape, Taylor approximations converging, unit-circle trig, complex-function domain colouring | `apply_matrix` exists but there is no basis / eigen / determinant overlay → **`linear_map_view`**. No epicycles → **DFT + `epicycles`**. No Taylor helper → **`taylor_coeffs` + partial sums**. → **unit circle**, **domain colouring** |
| **GeoGebra / Desmos** | Slope fields with solution curves, sliders, sequences, number-theory toys | → **`slope_field`** with RK4 solution curves; **Ulam spiral**, **modular times-table circles**, **Pascal-mod-p (Sierpinski)**, **Collatz** paths |
| **PhET simulations** | Wave interference ripple tank, ray optics with any elements, gravity & orbits, gas properties with live histograms, projectile, circuit construction | Waves are 1D only → **2D interference field**, **Doppler wavefronts**. Ray diagrams are fixed → general **2D ray tracer** (thin lenses, mirrors, refracting polygons and circles with Snell's law and total internal reflection). → **Kepler orbits** with equal-area sweeps, **gravity-well grids**. Gas sim exists without its statistics → **Maxwell–Boltzmann histogram**. → **phase portraits**, **chaos fan** of double pendulums |
| **Mathematica / Wolfram** | `StreamPlot`, `DensityPlot`, `ComplexPlot`, `ListAnimate`, cellular automata (`CellularAutomaton`) | → domain colouring, density images, **elementary CA (rule 30/90/110)** and **Game of Life** |
| **VPython** | Charged particles in fields, orbits, springs in 3D | → **Lorentz-force paths** (cyclotron / E×B drift) |
| **matplotlib / plotly animations** | Animated histograms, distributions, bar-chart races, heatmaps | → **`histogram`**, **Galton board**, **normal / binomial overlays**, **bar-chart race** |
| **Molecule viewers (Jmol, PyMOL, Avogadro)** and chemistry tools | Orbital iso-surfaces / densities, reaction coordinate diagrams, titration curves | Orbital *shapes* exist, not physics → **hydrogen orbital probability densities** from real wavefunctions (n, l, m). → **reaction energy profiles** with Ea / ΔH / catalysed path, **titration curves** (strong & weak acid) |
| **Epidemiology / ecology sims (NetLogo)** | Agent-based SIR, population dynamics | Only curves exist → baked **agent-based SIR** with matching curves |
| **Algorithm visualisers (VisuAlgo, sorting.at)** | Sorting bars with compare / swap highlights, BFS / DFS / Dijkstra frontier, tree layouts, neural networks lighting up | → **sort traces** (bubble, insertion, selection, quick, merge, heap), **graph search traces** (BFS, DFS, Dijkstra), **tidy tree layout** + BST insertion, **neural-network forward pass** |

Creative extras: a **"function machine"** view of Taylor polynomials racing the target,
**epicycles that draw text outlines**, a **chaos fan** of 30 double pendulums diverging
by 1e-4 rad, and a **light table** where rays bounce through a prism, lens and mirrors.

## Phases

| # | Phase | Module | Contents |
|---|---|---|---|
| S1 | Math visuals | `mathviz` | DFT + epicycles, linear-map view (transformed grid, basis vectors, eigenvectors, determinant patch), Taylor coefficients and partial sums, unit-circle trig, slope fields + solution curves, domain colouring, Ulam spiral, times-table circles, Pascal mod p, Collatz |
| S2 | Physics labs | `physviz` | 2D interference / ripple tank, Doppler wavefronts, 2D ray tracer (lenses, mirrors, refracting polygons/circles, TIR), Kepler orbits + equal-area sweeps, gravity-well grid, phase portraits, chaos fan, Maxwell–Boltzmann histogram, Lorentz-force paths |
| S3 | Chemistry & biology | `chembio` | Hydrogen orbital densities, reaction energy profiles, titration curves, agent-based SIR epidemics |
| S4 | CS & data | `algoviz` | Sort traces + bar frames, BFS / DFS / Dijkstra traces, tidy tree layout + BST, neural-network forward pass, histograms, Galton board, bar-chart race, cellular automata (elementary + Life) |
| S5 | Scripting & docs | `runwee-cli` | Rhai bindings, `examples/scripts/stem.rhai`, README |

As in the advanced layer, every visual is a pure function of time: traces and simulations
are computed once and sampled per frame.
