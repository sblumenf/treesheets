# TreeSheets Web Port: Consolidated Viability Study

## Executive Summary

This study assesses the feasibility of porting TreeSheets—a C++/wxWidgets hierarchical spreadsheet application—to a modern web stack. Two independent analyses were conducted and synthesized, resolving a critical technical discrepancy (framework identification) while combining complementary strengths in architectural planning, persistence strategy, and implementation roadmaps.

**Primary Finding**: A web port is technically viable using a **hybrid WebAssembly architecture** that compiles the C++ document model while rebuilding the entire UI layer in web-native technologies. Direct compilation of the full application is not feasible due to wxWidgets' lack of production-ready WebAssembly support.

**Estimated Effort**: 6–12 months with a team of 2–3 experienced developers.

**Key Risk**: The rendering layer cannot be compiled from C++; it must be rebuilt entirely, representing the critical path item.

---

## I. Application Analysis

### A. Correct Technical Foundation

TreeSheets is built on **wxWidgets**, not Qt. This distinction is architecturally decisive:

| Aspect | wxWidgets (Actual) | Qt (Incorrectly Assumed) |
|--------|-------------------|-------------------------|
| WebAssembly support | Experimental only; wxUniversal backend is "seriously unmaintained" | Official support since Qt 5.13 |
| Rendering API | wxDC (Device Context) | QPainter with OpenGL/DirectX acceleration |
| Web compilation path | Document model only; UI requires full rebuild | Potentially full-stack compilation |
| Precedent projects | GDevelop successfully migrated | Various Qt-to-WASM ports exist |

The wxWidgets reality eliminates the option of compiling the rendering pipeline directly to WebGL. The hybrid architecture is not merely recommended—it is the only viable path.

### B. Codebase Characteristics

The TreeSheets codebase exhibits characteristics favorable to porting:

- **Size**: Approximately 6,000–8,000 lines of dense, single-author C++ code
- **Structure**: Author describes it as "very small and simple, with all functionality easy to find and only in one place"
- **License**: ZLIB (permissive, allows derivative works)
- **Architecture**: Clean separation between document model and GUI layer

**Core Technical Components**:

| Component | Description | Portability |
|-----------|-------------|-------------|
| Recursive data model | Cell → Grid → Cell nesting to unlimited depth | High (pure C++ logic) |
| Binary serialization | Proprietary .cts format with zlib compression | High (compile to WASM) |
| Layout engine | Recursive size calculation for nested hierarchies | High (compile to WASM) |
| Rendering | wxDC-based 2D drawing with semantic zoom | Low (must rebuild) |
| Lobster scripting | Embedded language for automation/export | Medium (decision required) |
| GUI components | wxWidgets dialogs, menus, toolbars | Low (must rebuild) |

### C. Performance Baseline

The native application establishes a demanding performance benchmark:

- **Memory**: ~5MB on Windows 7 for documents equivalent to 100 pages
- **Responsiveness**: Fluid interaction with large, deeply nested hierarchies
- **File size**: Compact .cts files via custom binary serialization

This baseline is non-negotiable for user adoption. The web port must demonstrate comparable performance, particularly for:
- Document loading and parsing
- Recursive layout calculations
- Zoom and pan operations
- Large document rendering

---

## II. Architectural Decision: Hybrid WebAssembly

### A. Why Full Compilation Is Not Viable

Direct wxWidgets-to-WebAssembly compilation fails for structural reasons:

1. **wxUniversal limitation**: The wxWidgets web backend requires wxUniversal, which is unmaintained and produces frequent build errors
2. **No official support**: Unlike Qt, wxWidgets has no production WebAssembly target; an open feature request (Issue #18600) has remained unresolved since 2019
3. **Rendering dependency**: wxDC drawing operations have no direct WebGL translation path

### B. The Hybrid Architecture

The recommended architecture separates portable logic from platform-dependent rendering:

```
┌─────────────────────────────────────────────────────────────┐
│                    TypeScript Host Shell                     │
│  (React/SolidJS: menus, toolbars, dialogs, file handling)   │
└─────────────────────────┬───────────────────────────────────┘
                          │ Embind/WebIDL Bindings
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                 WebAssembly Core (~3MB)                      │
│  • Document model (Cell, Grid, Document classes)            │
│  • Binary .cts serialization/deserialization                │
│  • Layout calculation engine                                 │
│  • Undo/redo state management                               │
│  • (Optional) Lobster scripting runtime                     │
└─────────────────────────┬───────────────────────────────────┘
                          │ Render commands / Layout data
                          ▼
┌─────────────────────────────────────────────────────────────┐
│              Canvas/WebGL Rendering Layer                    │
│  (New implementation: hierarchical grid, semantic zoom)      │
└─────────────────────────────────────────────────────────────┘
```

**What gets compiled to WASM**:
- `Cell`, `Grid`, `Document` class hierarchies
- .cts file parser and serializer
- Recursive layout algorithms
- Clipboard and selection logic
- Undo/redo via document snapshots

**What gets rebuilt in TypeScript/Canvas**:
- All wxDC rendering operations → Canvas 2D or WebGL
- wxWidgets dialogs → React/SolidJS components
- Menu and toolbar systems
- File picker integration
- Keyboard/mouse event handling

### C. Precedent Validation: GDevelop

GDevelop provides direct validation of this approach. Originally a C++/wxWidgets game development IDE, it successfully migrated to web using the identical pattern:

1. Stripped wxWidgets GUI from C++ codebase
2. Compiled core classes (scene management, object handling) to ~3MB WASM via Emscripten
3. Created WebIDL bindings exposing C++ API to JavaScript
4. Built new React frontend with Material UI components

GDevelop now runs entirely in-browser while maintaining its C++ game engine core. This demonstrates that wxWidgets applications can reach the web—but the GUI layer must be rewritten entirely.

---

## III. Technology Stack Recommendations

### A. Rendering Layer

Standard DOM/CSS rendering is insufficient for TreeSheets' requirements. The nested hierarchical grid demands:

- Pixel-accurate placement
- Multi-level hierarchy rendering
- Dynamic resizing
- High-speed zoom and pan
- "Tiny" rendering mode (1 pixel per character)

**Recommended approach**: Canvas 2D with WebGL acceleration for complex operations

| Option | Pros | Cons | Recommendation |
|--------|------|------|----------------|
| Custom Canvas 2D | Full control, lightweight | More development effort | **Primary choice** |
| Konva.js | Layer abstraction, event handling | Additional dependency | Consider for acceleration |
| PixiJS | Excellent performance (60 FPS @ 8,000 objects) | Overkill for 2D grids | Only if performance issues arise |
| DOM-based | Standard tooling | Performance ceiling at ~1,000-5,000 elements | Not viable |

**Virtualization is mandatory**: Only visible cells should be rendered. TanStack Virtual or similar libraries achieve 60 FPS scrolling with 100,000+ rows. The recursive TreeSheets structure requires adapting standard row virtualization to handle nested depth.

### B. Application Framework

| Framework | Strengths for TreeSheets | Considerations |
|-----------|-------------------------|----------------|
| **SolidJS** | Fine-grained reactivity updates only exact leaf nodes; no reconciliation overhead for nested structures | Smaller ecosystem than React |
| React | Largest ecosystem, extensive tooling | Virtual DOM reconciliation expensive for deep nesting |
| Vue 3 | Good reactivity model | Middle ground; no clear advantage |

**Recommendation**: SolidJS for its superior handling of deeply nested reactive data structures, or React if team expertise favors it.

### C. State Management

TreeSheets' deeply nested document model requires specialized state handling:

| Solution | Approach | Fit for TreeSheets |
|----------|----------|-------------------|
| **Zustand + Immer** | Immutable updates with structural sharing; only changed tree portions copied | **Recommended** |
| MobX-Keystone | Built-in hierarchical data handling and undo | Strong alternative |
| Redux Toolkit | Normalized state, Immer integration | Overkill for document-centric app |

**Undo/Redo**: Implement via zundo (Zustand middleware) or JSON patch-based system for memory-efficient history.

### D. WASM Toolchain

- **Compiler**: Emscripten (emcc) with Clang/LLVM backend
- **Bindings**: Embind for exposing C++ classes to JavaScript
- **Optimization flags**: `-Os --closure 1` for aggressive size reduction
- **Target**: wasm32 with JavaScript glue code generation

---

## IV. Persistence and File Management

### A. Proprietary .cts File Format

The hybrid architecture preserves .cts compatibility by compiling the existing C++ serialization routines:

```
JavaScript File Picker → ArrayBuffer → WASM Module → Parsed Document
                                              ↓
                                    WASM Module → ArrayBuffer → Download
```

This approach:
- Eliminates risk of serialization mismatches
- Preserves compact file sizes
- Maintains backward/forward compatibility with desktop version

### B. File System Access API Constraints

A critical browser limitation affects user experience:

| Capability | Chrome/Edge Desktop | Firefox | Safari | Mobile |
|------------|--------------------| --------|--------|--------|
| Open file dialog | ✅ | ✅ (traditional) | ✅ (traditional) | ✅ |
| Save to same file | ✅ | ❌ | ❌ | ❌ |
| Persist file handles | ✅ | ❌ | ❌ | ❌ |
| Recent files list | ✅ | ❌ | ❌ | ❌ |

**User experience implication**: On Firefox, Safari, and mobile browsers, users must explicitly download a new .cts file for each save operation. This is a significant departure from desktop expectations and must be clearly communicated in the UI.

**Mitigation**: Use the browser-fs-access library for graceful degradation across browsers.

### C. IndexedDB for Internal State

Given FSA API limitations, internal persistence (autosave, undo history, offline access) must use IndexedDB:

**Anti-pattern**: Writing the entire document tree on every change
- Blocks main thread
- Creates perceptible lag
- Wastes storage writes

**Recommended pattern**: Transactional diff-based persistence
```javascript
// Only persist the minimal change-set
async function persistChange(documentId, patch) {
  const tx = db.transaction(['documents', 'patches'], 'readwrite');
  await tx.objectStore('patches').add({
    documentId,
    timestamp: Date.now(),
    patch // JSON Patch or similar diff format
  });
  // Periodically compact patches into full snapshots
}
```

This approach:
- Preserves UI responsiveness during autosave
- Enables granular undo/redo recovery
- Reduces IndexedDB write volume

---

## V. Asynchronous Architecture Considerations

### A. The Sync/Async Impedance Mismatch

Desktop C++ applications commonly use synchronous patterns that browsers prohibit:

| Desktop Pattern | Browser Constraint | Required Adaptation |
|-----------------|-------------------|---------------------|
| Modal dialogs (`wxDialog::ShowModal()`) | Cannot block main thread | Convert to async Promise-based flow |
| Nested event loops | Single-threaded JS model | Restructure control flow |
| Synchronous file I/O | All I/O is async | Use async/await patterns |
| Blocking drag-and-drop | Event-driven model | Refactor to callbacks |

### B. Asyncify: Avoid If Possible

Emscripten's Asyncify tool enables synchronous C++ code to await JavaScript Promises by post-processing WASM to suspend/resume the execution stack.

**Costs of Asyncify**:
- Longer build times
- Larger binary sizes (10-20% increase typical)
- Runtime performance overhead
- Increased debugging complexity

**Recommendation**: Refactor synchronous C++ patterns before compilation rather than relying on Asyncify. This adds complexity to the porting phase but preserves runtime performance.

### C. JSPI as Future Alternative

WebAssembly JS Promise Integration (JSPI) offers Asyncify-like capability with minimal overhead. However, browser support is not yet universal:

| Browser | JSPI Status |
|---------|-------------|
| Chrome | Origin trial / flag |
| Firefox | In development |
| Safari | Not yet supported |

**Recommendation**: Design for explicit async refactoring now; JSPI may simplify future iterations.

---

## VI. The Lobster Scripting Decision

TreeSheets includes an embedded scripting language (Lobster) by the same author, used for:
- Custom export formats
- Document automation
- User-defined operations

**Options**:

| Approach | Pros | Cons |
|----------|------|------|
| Compile Lobster to WASM | Preserves existing scripts; maintains compatibility | Adds to bundle size; complex integration |
| Replace with JavaScript | Native web language; easier maintenance | Breaks existing scripts; migration burden for power users |
| Drop scripting initially | Reduces scope; faster initial release | Feature regression; may alienate power users |

**Recommendation**: Phase scripting support. Launch without Lobster; add JavaScript-based automation in a later release. Document the decision and provide migration guidance for existing script users.

---

## VII. Risk Assessment Matrix

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Canvas rendering performance doesn't match native | Medium | High | Profile early; budget for WebGL acceleration if needed |
| Text measurement parity (wxDC → Canvas) | High | Medium | Allocate extra development time; test across fonts |
| WASM bundle size exceeds acceptable threshold | Medium | Medium | Aggressive optimization (-Os, tree shaking); lazy loading |
| Recursive layout performance at scale | Medium | High | Implement incremental calculation; cache layout results |
| Cross-browser FSA API fragmentation | Certain | Medium | Clear UX communication; download-based fallback |
| Lobster scripting compatibility | Low | Low | Defer to later phase |
| IndexedDB quota limits on large documents | Low | Medium | Implement storage monitoring; warn users proactively |
| Accessibility for Canvas content | Medium | Medium | Implement hidden parallel DOM from Phase 1 |

---

## VIII. Effort Estimation

### A. Team Composition

| Role | Skills Required | Allocation |
|------|-----------------|------------|
| C++/WASM Engineer | C++, Emscripten, WebAssembly bindings | 1 developer |
| Frontend Engineer | TypeScript, React/SolidJS, Canvas rendering | 1-2 developers |
| (Optional) Graphics Specialist | WebGL, performance optimization | Part-time/consulting |

### B. Timeline Summary

| Phase | Duration | Key Deliverable |
|-------|----------|-----------------|
| Phase 1: POC | 4-6 weeks | WASM core loading .cts files; basic Canvas rendering |
| Phase 2: Architecture | 6-10 weeks | Async refactoring; IndexedDB persistence; stable API |
| Phase 3: Feature Parity | 10-16 weeks | Full UI rebuild; all desktop features |
| Phase 4: Polish | 2-4 weeks | PWA, performance audit, launch preparation |
| **Total** | **22-36 weeks** | **~6-9 months** |

Add 20-30% contingency for unforeseen complexity, yielding **6-12 months** total estimate.

---

## IX. Implementation Roadmap

### Phase 1: Performance and I/O Proof of Concept (4-6 weeks)

**Objective**: Validate core WASM execution speed and .cts file integrity under web constraints.

**Milestones**:

1. **C++ separation** (Week 1-2)
   - Fork TreeSheets repository
   - Identify and isolate document model classes from wxWidgets dependencies
   - Create `libTreeSheets` with clean compilation boundary
   - Verify desktop build still functions with refactored structure

2. **Emscripten compilation** (Week 2-3)
   - Configure Emscripten toolchain
   - Compile `libTreeSheets` to WASM with `-Os --closure 1`
   - Generate Embind bindings for Cell, Grid, Document classes
   - Target bundle size: <3MB compressed

3. **File I/O validation** (Week 3-4)
   - Implement JavaScript bridge for file picker → WASM
   - Load and parse representative .cts files (small, medium, large)
   - Verify data integrity via round-trip serialization
   - Benchmark parsing performance against desktop baseline

4. **Rendering POC** (Week 4-6)
   - Implement minimal Canvas renderer for flat grid
   - Add single-level nesting support
   - Render large .cts file; measure frame rate during pan/zoom
   - **Go/No-Go Decision**: If rendering performance is unacceptable, evaluate WebGL acceleration or scope reduction

**Deliverables**:
- Compilable WASM module with JavaScript bindings
- Basic HTML test harness demonstrating file load and render
- Performance benchmark report comparing to desktop baseline
- Technical risk assessment update

### Phase 2: Architectural Decoupling and Persistence (6-10 weeks)

**Objective**: Refactor C++ synchronous patterns and implement robust web persistence.

**Milestones**:

1. **Async refactoring** (Week 1-4)
   - Audit C++ codebase for synchronous wxWidgets API calls
   - Refactor modal dialogs to callback/Promise patterns
   - Restructure drag-and-drop to event-driven model
   - Eliminate nested event loops
   - Recompile and verify no Asyncify dependency

2. **API stabilization** (Week 3-5)
   - Define complete Embind interface for all document operations
   - Optimize data marshaling for complex structural updates
   - Implement efficient change notification from WASM to JS
   - Document API contract for frontend development

3. **IndexedDB persistence** (Week 4-7)
   - Implement transactional storage layer
   - Design diff-based change persistence (JSON Patch or similar)
   - Add periodic snapshot compaction
   - Implement offline detection and sync queue

4. **File handling layer** (Week 6-8)
   - Integrate browser-fs-access library
   - Implement FSA API for Chrome/Edge with full save capability
   - Implement download fallback for Firefox/Safari
   - Design "recent files" feature using IndexedDB for handle persistence

**Deliverables**:
- Async-clean WASM module (no Asyncify overhead)
- Stable, documented JavaScript API
- IndexedDB persistence layer with diff-based autosave
- Cross-browser file handling abstraction

### Phase 3: Feature Parity and UI Development (10-16 weeks)

**Objective**: Rebuild complete TreeSheets UI in web technologies.

**Milestones**:

1. **Core rendering engine** (Week 1-6)
   - Implement hierarchical Canvas renderer with unlimited nesting
   - Add semantic zoom (document root navigation)
   - Implement "tiny" rendering mode for large documents
   - Add virtualization for visible-cell-only rendering
   - Achieve 60 FPS target for pan/zoom operations

2. **Cell editing** (Week 4-8)
   - Implement DOM overlay for text editing
   - Support all text formatting options (bold, italic, colors)
   - Add image embedding within cells
   - Implement cell resize handles

3. **Application shell** (Week 6-10)
   - Build menu system (File, Edit, View, etc.)
   - Implement toolbar with all desktop actions
   - Create settings/preferences dialog
   - Add keyboard shortcut system matching desktop

4. **Advanced features** (Week 8-14)
   - Presentation mode
   - Search and replace across document
   - Import/export formats (HTML, text, CSV)
   - Clipboard integration (internal and system)
   - Undo/redo UI with history visualization

5. **Accessibility** (Week 10-14)
   - Implement hidden parallel DOM for screen readers
   - Add keyboard navigation for all operations
   - Test with NVDA, VoiceOver, JAWS
   - Address WCAG 2.1 AA compliance gaps

**Deliverables**:
- Feature-complete web application
- Accessibility audit report
- Cross-browser testing results (Chrome, Firefox, Safari, Edge)
- Performance benchmark suite

### Phase 4: PWA and Production Preparation (2-4 weeks)

**Objective**: Production-ready deployable application.

**Milestones**:

1. **PWA configuration** (Week 1-2)
   - Create web app manifest
   - Implement Service Worker for offline capability
   - Add install prompts for desktop/mobile
   - Configure caching strategy for WASM module

2. **Performance optimization** (Week 1-2)
   - Implement parallel loading (WASM + JS simultaneously)
   - Add loading progress indicator
   - Optimize initial render path (show content before full load)
   - Profile and address memory leaks

3. **Launch preparation** (Week 2-4)
   - Final cross-browser testing
   - Documentation (user guide, keyboard shortcuts)
   - Analytics integration (privacy-respecting)
   - Error reporting setup (Sentry or similar)

**Deliverables**:
- Production-deployed PWA
- User documentation
- Monitoring and error reporting infrastructure
- Launch announcement materials

---

## X. Alternatives Considered

### A. Electron Wrapper

Wrapping the existing C++ application in Electron would produce a downloadable desktop app, not a web app. This defeats the cross-platform, zero-install value proposition.

**Verdict**: Does not meet requirements.

### B. Tauri

Tauri offers smaller bundles (3-10MB) than Electron but requires a Rust backend. Integrating TreeSheets' C++ code demands FFI bridges, adding complexity without enabling true web deployment.

**Verdict**: Suitable for desktop distribution; does not meet web deployment requirement.

### C. Full TypeScript Rewrite

Reimplementing the entire application in TypeScript eliminates C++ complexity but:
- Risks 100-200x performance regression for computational operations
- Requires reverse-engineering the proprietary .cts format
- Extends timeline to 12-18 months
- Loses battle-tested C++ logic

**Verdict**: Higher risk, longer timeline, no clear benefit over hybrid approach.

### D. Wait for wxWidgets WASM Support

The wxWidgets WebAssembly feature request has been open since 2019 with no production-ready implementation.

**Verdict**: Indefinite delay; not a viable strategy.

---

## XI. Conclusion and Recommendation

### Viability Assessment: **VIABLE WITH DEFINED CONSTRAINTS**

A web port of TreeSheets is technically achievable using the hybrid WebAssembly architecture. The approach is validated by GDevelop's successful migration from the identical technology stack (C++/wxWidgets → WASM + React).

### Critical Success Factors

1. **Rendering performance**: The Canvas-based UI must match desktop responsiveness. Early profiling and potential WebGL acceleration are essential.

2. **Scope management**: Defer Lobster scripting to post-launch. Focus on core hierarchical spreadsheet functionality.

3. **UX honesty**: Clearly communicate File System Access API limitations. Don't promise desktop-like saving on unsupported browsers.

4. **Team expertise**: Requires C++/Emscripten skills (rare) alongside modern frontend expertise. Budget for learning curve or specialist consulting.

### Final Recommendation

**Proceed with Phase 1 POC** to validate rendering performance and WASM compilation before committing to full development. The 4-6 week investment will confirm technical feasibility with minimal risk exposure.

If POC metrics meet baseline targets, proceed with full implementation on the 6-12 month timeline. If rendering performance is unacceptable, reassess scope (e.g., limit maximum document size) or evaluate WebGL acceleration investment.

---

## Appendix A: Technology Reference

| Component | Recommended | Alternatives |
|-----------|-------------|--------------|
| WASM Compiler | Emscripten | None viable for wxWidgets |
| JS Bindings | Embind | WebIDL |
| Frontend Framework | SolidJS | React, Vue 3 |
| State Management | Zustand + Immer | MobX-Keystone |
| Rendering | Custom Canvas 2D | Konva, PixiJS |
| Virtualization | TanStack Virtual | react-window |
| File Handling | browser-fs-access | Native FSA API |
| Persistence | IndexedDB (idb library) | Dexie.js |
| Undo/Redo | zundo, JSON Patch | Immer patches |

## Appendix B: Key Resources

- TreeSheets Repository: https://github.com/aardappel/treesheets
- GDevelop Architecture: https://github.com/4ian/GDevelop
- Emscripten Documentation: https://emscripten.org/docs/
- wxWidgets WASM Issue: https://github.com/wxWidgets/wxWidgets/issues/18600
- Figma Engineering Blog: https://www.figma.com/blog/building-a-professional-design-tool-on-the-web/
- browser-fs-access: https://github.com/nicknisi/browser-fs-access
