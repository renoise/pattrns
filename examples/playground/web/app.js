import { default as createModule } from "./playground.js";

// -------------------------------------------------------------------------------------------------

const defaultBpm = 120;
const defaultInstrument = 0;
const defaultScriptContent = `--
-- Welcome to the pattrns playground!
--
-- Create and experiment with pattern scripts here to learn how they work.
-- Check out the interactive 'Quickstart' scripts on the right or load examples
-- to get started.
--
-- **Nothing is persistent here**: Copy and save scripts locally to keep them!
--
-- The playground uses a simple sample player backend. The currently selected 
-- sample plays by default, unless your script specifies an instrument explicitely.
--
-- Use 'CTRL+Return' or 'CTRL+S' to apply script changes.
-- Use 'CTRL+SHIFT+SPACE' to start/stop playing.
--

-- the note patterns that we're emitting
local note_patterns = {
  { "c5", "d#5", "c6", "g5", "c5", "g#4" },
  { "c4", "d#4", "f4", "c5", "d#5", "f5" },
  { "d#5", "g5", "d#5", "c6", "g#4", "c5" },
  { "d#4", "c4", "f4", "c5", "f4", "d#5"  },
}

-- get arp direction step sign from the given direction parameter mode string
local function step_sign(direction)
  if direction == "up" then return 1
  elseif direction == "down" then return -1
  else -- random
    return math.random() > 0.5 and 1 or -1
  end
end

-- create final pattern
return pattern {
  unit = "1/16",
  parameter = {
    parameter.integer("pattern_length", 48, { 1, 256 }, "Pattern Length", 
      "How often, in steps, we play a single arp pattern."),
    parameter.enum("direction", "up", { "up", "down", "random" }, "Arp Direction", 
      "How to move through a single arp pattern."),
    parameter.number("mod_amount", 0.25, { 0, 1 }, "Mod Amount",
      "Vol/pan modulation amount."),
    parameter.integer("mod_length", 24, { 1, 256 }, "Mod Length",
      "Vol/pan modulation length in unit steps."),
  },
  pulse = { 1.0, 0.25, 0.8, 0.6, 0.4 },
  event = function(context)
    local pattern_length, direction, mod_amount, mod_length = 
      context.parameter.pattern_length, context.parameter.direction, 
      context.parameter.mod_amount, context.parameter.mod_length
    local pattern_step = math.imod(math.floor((context.step - 1) / pattern_length) + 1, #note_patterns)
    local notes = note_patterns[pattern_step]
    local note_step = math.imod(step_sign(direction) * context.step, #notes)
    local vmod = math.cos(context.step / mod_length * math.pi)
    local pmod = math.sin(context.step / mod_length / 3 * math.pi)
    return {
        key = notes[note_step], 
        volume = context.pulse_value * (0.5 + 0.5 * mod_amount * vmod),
        panning = mod_amount * pmod,
        instrument = nil
    }
  end
}
`;

// -------------------------------------------------------------------------------------------------

const backend = {
    _playground: undefined,
    _isPlaying: false,

    initialize: function (playground) {
        this._playground = playground;

        const err = this._playground.ccall('initialize_playground', 'string', [])
        if (err) {
            return err
        }

        this.updateBpm(defaultBpm);
        this.updateInstrument(defaultInstrument);
        this.updateScriptContent(defaultScriptContent);

        return undefined;
    },

    getSamples: function () {
        const stringPtr = this._playground.ccall('get_samples', 'number', [])
        const samplesJson = this._playground.UTF8ToString(stringPtr);
        this._freeCString(stringPtr)
        return JSON.parse(samplesJson);
    },

    getQuickstartScripts: function () {
        const stringPtr = this._playground.ccall('get_quickstart_scripts', 'number', [])
        const examplesJson = this._playground.UTF8ToString(stringPtr);
        this._freeCString(stringPtr)
        return JSON.parse(examplesJson);
    },

    getExampleScripts: function () {
        const stringPtr = this._playground.ccall('get_example_scripts', 'number', [])
        const examplesJson = this._playground.UTF8ToString(stringPtr);
        this._freeCString(stringPtr)
        return JSON.parse(examplesJson);
    },

    getScriptError: function () {
        let stringPtr = this._playground.ccall('get_script_error', 'number', [])
        const error = this._playground.UTF8ToString(stringPtr);
        this._freeCString(stringPtr)
        return error;
    },

    getScriptParameters: function () {
        let stringPtr = this._playground.ccall('get_script_parameters', 'number', [])
        const json = this._playground.UTF8ToString(stringPtr);
        const parameters = JSON.parse(json);
        this._freeCString(stringPtr)
        return parameters;
    },

    isPlaying: function () {
        return this._isPlaying;
    },

    startPlaying: function () {
        this._playground.ccall("start_playing");
        this._isPlaying = true;
    },

    stopPlaying: function () {
        this._playground.ccall("stop_playing");
        this._isPlaying = false;
    },

    setVolume: function (volume) {
        this._playground.ccall("set_volume", "undefined", ["number"], [volume]);
    },

    stopPlayingNotes: function () {
        this._playground.ccall("stop_playing_notes");
    },

    sendMidiNoteOn: function (note, velocity) {
        this._playground.ccall("midi_note_on", 'undefined', ['number', 'number'], [note, velocity]);
    },

    sendMidiNoteOff: function (note) {
        this._playground.ccall("midi_note_off", 'undefined', ['number'], [note]);
    },

    updateInstrument: function (instrument) {
        this._playground.ccall("set_instrument", 'undefined', ['number'], [instrument]);
    },

    updateBpm: function (bpm) {
        this._playground.ccall("set_bpm", 'undefined', ['number'], [bpm]);
    },

    updateParameterValue: function (id, value) {
        this._playground.ccall("set_parameter_value", "undefined", ["string", "number"], [id, value]);
    },

    updateScriptContent: function (content) {
        this._playground.ccall("update_script", 'undefined', ['string'], [content]);
    },

    loadSample: function (filename, buffer) {
        const data = new Uint8Array(buffer);
        const newSampleId = this._playground.ccall(
            'load_sample',
            'number',
            ['string', 'array', 'number'],
            [filename, data, data.length]
        );
        return newSampleId;
    },

    clearSamples: function () {
        this._playground.ccall('clear_samples', 'undefined', []);
    },

    _freeCString: function (stringPtr) {
        this._playground.ccall('free_cstring', 'undefined', ['number'], [stringPtr])
    },
};

// -------------------------------------------------------------------------------------------------

const app = {
    _initialized: false,
    _editor: undefined,
    _editCount: 0,
    _changedHashFromUserEdit: false,
    _changedScriptFromHash: false,

    initialize: function () {
        // hide spinner, show content
        let splash = document.getElementById('loading-splash');
        let content = document.getElementById('app-content');
        console.assert(splash && content);
        splash.style.display = 'none';
        content.style.display = 'flex';
        // init components
        this._initialized = true;
        this._initControls();
        this._initSampleDropdown();
        this._initExampleScripts();
        this._initScriptErrorHandler();
        this._initScriptParameterHandler();
        this._initEditor();
    },

    // Show status message in loading screen or status bar
    setStatus: function (message, isError) {
        // log to console
        (isError ? console.error : console.log)(message);
        // update app text
        const statusElement = this._initialized
            ? document.getElementById('status')
            : document.getElementById('spinner-status');
        if (statusElement != undefined) {
            statusElement.textContent = message.replace(/(?:\r\n|\r|\n)/g, '\t');
            statusElement.style.color = isError ? 'var(--color-error)' : 'var(--color-success)';
            // clear non-error messages after 5 seconds
            if (this._clearStatusTimeout) {
                clearTimeout(this._clearStatusTimeout);
                this._clearStatusTimeout = null;
            }
            if (!isError) {
                this._clearStatusTimeout = setTimeout(() => {
                    statusElement.textContent = '';
                }, 5000);
            }
        }
    },

    // Init transport controls
    _initControls: function () {
        // Set up control handlers
        const playButton = document.getElementById('playButton');
        const stopButton = document.getElementById('stopButton');
        const midiButton = document.getElementById('midiButton');
        console.assert(playButton && stopButton && midiButton);

        playButton.addEventListener('click', () => {
            backend.startPlaying();
            this.setStatus("Playing...");
            playButton.style.color = 'var(--color-accent)';
        });
        stopButton.addEventListener('click', () => {
            backend.stopPlaying();
            this.setStatus("Stopped");
            playButton.style.color = null;
        });

        const bpmInput = document.getElementById('bpmInput');
        console.assert(bpmInput);

        bpmInput.min = 20;
        bpmInput.max = 999;
        bpmInput.addEventListener('change', (e) => {
            const bpm = parseInt(e.target.value);
            if (!isNaN(bpm)) {
                const clampedBpm = Math.max(bpmInput.min, Math.min(bpm, bpmInput.max));
                if (bpm !== clampedBpm) {
                    e.target.value = clampedBpm;
                }
                backend.updateBpm(clampedBpm);
                this.setStatus(`Set new BPM: '${clampedBpm}'`);
            }
        });

        const volumeSlider = document.getElementById('volumeSlider');
        const volumeInput = document.getElementById('volumeInput');
        console.assert(volumeSlider && volumeInput);

        function updateVolumeDisplay(gain) {
            // Update slider
            volumeSlider.value = Math.round(gain * 100);
            // Update text input
            if (gain <= 0.0001) {
                volumeInput.value = '-INF dB';
            } else {
                const db = 20 * Math.log10(gain);
                volumeInput.value = `${db.toFixed(1)} dB`;
            }
        }

        // Initial setup
        const initialGain = parseInt(volumeSlider.value, 10) / 100.0;
        backend.setVolume(initialGain);
        updateVolumeDisplay(initialGain);

        // Event listener for slider
        volumeSlider.addEventListener('input', (e) => {
            const gain = parseInt(e.target.value, 10) / 100.0;
            backend.setVolume(gain);
            updateVolumeDisplay(gain);
        });

        // Event listener for text input
        volumeInput.addEventListener('change', (e) => {
            let dbString = e.target.value.trim();
            let db = dbString.toLowerCase().includes('-inf')
                ? -Infinity
                : parseFloat(dbString);
            if (isNaN(db)) {
                // Invalid input, revert to current slider value
                const currentGain = parseInt(volumeSlider.value, 10) / 100.0;
                updateVolumeDisplay(currentGain);
                return;
            }
            db = Math.min(db, 3.0); // Clamp to +3dB
            const gain = isFinite(db) ? Math.pow(10, db / 20) : 0;
            // apply
            backend.setVolume(gain);
            updateVolumeDisplay(gain);
        });

        // When focusing the input, remove " dB" for easier editing
        volumeInput.addEventListener('focus', (e) => {
            e.target.value = e.target.value.replace(/\s*dB/i, '');
        });
        volumeInput.addEventListener('blur', (e) => {
            // Trigger change event to re-validate and re-format
            e.target.dispatchEvent(new Event('change', { 'bubbles': true }));
        });

        let midiAccess = null;
        let midiEnabled = false;
        let currentMidiNotes = new Set();

        function enableMidi() {
            if (!navigator.requestMIDIAccess) {
                return Promise.reject(new Error("Web MIDI API not supported"));
            }
            return navigator.requestMIDIAccess()
                .then(access => {
                    midiAccess = access;
                    midiEnabled = true;
                    midiButton.style.color = 'var(--color-accent)';
                    // Start listening to MIDI input
                    for (let input of midiAccess.inputs.values()) {
                        input.onmidimessage = handleMidiMessage;
                    }
                    // stop regular playback
                    if (backend.isPlaying()) {
                        backend.stopPlaying();
                        playButton.style.color = null;
                    }
                    app.setStatus("MIDI input enabled. Press one or more notes on your keyboard to play the script...");
                });
        }

        function disableMidi() {
            midiEnabled = false;
            midiButton.style.color = null;
            // Stop listening to MIDI input
            if (midiAccess) {
                for (let input of midiAccess.inputs.values()) {
                    input.onmidimessage = null;
                }
            }
            // Release all notes
            currentMidiNotes.forEach(note => {
                backend.sendMidiNoteOff(note);
            });
            currentMidiNotes.clear();
            app.setStatus("MIDI input disabled");
            return Promise.resolve();
        }

        function handleMidiMessage(message) {
            const data = message.data;
            const status = data[0] & 0xF0;
            const note = data[1];
            const velocity = data[2];
            if (status === 0x90 && velocity > 0) { // Note on
                if (!currentMidiNotes.has(note)) {
                    currentMidiNotes.add(note);
                    backend.sendMidiNoteOn(note, velocity);
                }
            } else if (status === 0x80 || (status === 0x90 && velocity === 0)) { // Note off
                if (currentMidiNotes.has(note)) {
                    currentMidiNotes.delete(note);
                    backend.sendMidiNoteOff(note);
                }
            }
        }

        midiButton.addEventListener('click', () => {
            if (!midiEnabled) {
                enableMidi().then(() => {
                    // Disable play/stop buttons on success
                    playButton.disabled = true;
                    stopButton.disabled = true;
                }).catch(err => {
                    const isError = true;
                    app.setStatus("Failed to access MIDI: " + err, isError);
                });
            } else {
                disableMidi().then(() => {
                    // Re-enable play/stop buttons
                    playButton.disabled = false;
                    stopButton.disabled = false;
                }).catch(err => {
                    const isError = true;
                    app.setStatus("Failed to release MIDI: " + err, isError);
                });
            }
        });

        const loadSampleButton = document.getElementById('loadSampleButton');
        const sampleFileInput = document.getElementById('sampleFileInput');
        const clearSamplesButton = document.getElementById('clearSamplesButton');
        console.assert(loadSampleButton && sampleFileInput && clearSamplesButton);

        loadSampleButton.addEventListener('click', () => {
            sampleFileInput.value = null;
            sampleFileInput.click();
        });

        clearSamplesButton.addEventListener('click', () => {
            backend.clearSamples();
            this.setStatus('All samples cleared.');
            this._initSampleDropdown();
        });

        sampleFileInput.addEventListener('change', (event) => {
            const file = event.target.files[0];
            if (!file) {
                return;
            }

            const maxSize = 4 * 1024 * 1024; // 4MB
            if (file.size > maxSize) {
                const isError = true;
                this.setStatus(`File '${file.name}' is too large. Maximum size is 4MB.`, isError);
                return;
            }

            let reader = new FileReader();
            reader.onload = (e) => {
                const buffer = e.target.result;
                const newId = backend.loadSample(file.name, buffer);
                if (newId >= 0) {
                    this.setStatus(`Loaded sample '${file.name}'`);
                    this._initSampleDropdown();
                    const select = document.getElementById('sampleSelect');
                    select.value = newId;
                    backend.updateInstrument(newId);
                } else {
                    const isError = true;
                    this.setStatus(`Failed to load sample '${file.name}'. The format may not be supported.`, isError);
                }
            };
            reader.onerror = (e) => {
                const isError = true;
                this.setStatus(`Error reading file: ${e.target.error.message}`, isError);
            };
            reader.readAsArrayBuffer(file);
        });
    },

    // Populate sample dropdown
    _initSampleDropdown: function () {
        const samples = backend.getSamples();

        const select = document.getElementById('sampleSelect');
        console.assert(select);

        select.innerHTML = '';
        if (samples.length > 0) {
            samples.forEach((sample, index) => {
                const option = document.createElement('option');
                option.value = sample.id;
                option.textContent = `${String(index).padStart(2, '0')}: ${sample.name}`;
                select.appendChild(option);
            });
            select.onchange = (event) => {
                let id = event.target.value;
                backend.updateInstrument(Number(id));
                this.setStatus(`Set new default instrument: '${id}'`);

            };

            // set last sample as default instrument
            select.value = samples[samples.length - 1].id
            backend.updateInstrument(select.value)
        } else {
            const option = document.createElement('option');
            option.value = 'none';
            option.textContent = 'No samples loaded';
            select.appendChild(option);
            select.onchange = null;
            backend.updateInstrument(-1);
        }
    },

    _updateScript: function(script, name = "untitled") {
        if (backend.isPlaying()) {
            backend.stopPlaying();
            backend.updateScriptContent(script);
            backend.startPlaying();
        } else {
            backend.updateScriptContent(script);
        }
        this._editor.setScrollPosition({ scrollTop: 0 });
        this._updateEditCount(0);
        this.setStatus(`Loaded script: '${name}'.`);
    },

    // Set up example scripts list
    _initExampleScripts: function () {
        const examples = backend.getExampleScripts();
        const quickstartExamples = backend.getQuickstartScripts();

        const examplesList = document.getElementById('examples-list');
        examplesList.innerHTML = '';

        // Add quickstart examples
        const quickstartSection = document.createElement('h3');
        quickstartSection.textContent = "Quickstart";
        examplesList.appendChild(quickstartSection);

        let allLinks = [];
        let loadExample = (link, example) => {
            allLinks.forEach(link => {
                link.style.textDecoration = 'none';
            });
            link.style.textDecoration = 'underline';
            this._updateScript(example.content, example.name);
        };

        const appendExampleLink = (example) => {
            const li = document.createElement('li');
            const a = document.createElement('a');
            a.href = `#${this._encodeScript(example.content)}`;
            a.textContent = example.name;
            a.style.color = 'var(--color-link)';
            a.style.textDecoration = 'none';
            a.onclick = () => loadExample(a, example);
            allLinks.push(a)
            li.appendChild(a);
            examplesList.appendChild(li);
        }

        quickstartExamples.forEach(group => {
            const quickstartGroup = document.createElement('h4');
            quickstartGroup.textContent = group.name;
            examplesList.appendChild(quickstartGroup);

            group.entries.forEach(appendExampleLink);
        });

        // Add examples
        const examplesSection = document.createElement('h3');
        examplesSection.textContent = "Examples";
        examplesList.appendChild(examplesSection);

        examples.forEach(appendExampleLink);
    },

    _encodeScript: function (script) {
        return btoa(JSON.stringify({ script }));
    },
    
    _decodeScriptFromHash: function () {
        const hash = window.location.hash;
        if (hash.length < 2) {
            return null;
        }
        try {
            const string = atob(hash.substring(1).split('?')[0]);
            const object = JSON.parse(string)
            return object.script;
        } catch (e) {
            return null;
        }
    },

    // Initialize Monaco editor
    _initEditor: function () {
        require.config({ paths: { 'vs': 'https://cdnjs.cloudflare.com/ajax/libs/monaco-editor/0.52.2/min/vs' } });

        let editorElement = document.getElementById('editor');
        console.assert(editorElement);


        require(['vs/editor/editor.main'], () => {
            // Try parsing script from URL hash or use the default
            const scriptContent = this._decodeScriptFromHash() || defaultScriptContent;
            
            // Create editor
            this._editor = monaco.editor.create(editorElement, {
                value: scriptContent,
                language: 'lua',
                theme: 'vs-dark',
                minimap: { enabled: false },
                scrollBeyondLastLine: false,
                automaticLayout: true,
                wordWrap: 'on',
                acceptSuggestionOnCommitCharacter: true
            });

            this._updateScript(scriptContent);

            // Track edits
            this._editor.onDidChangeModelContent(() => {
                if (this._changedScriptFromHash) {
                    this._changedScriptFromHash = false;
                    return;
                }
                this._changedHashFromUserEdit = true;
                window.location.hash = this._encodeScript(this._editor.getValue());
                this._updateEditCount(this._editCount + 1);
            });
            
            // Handle Ctrl+Enter
            const commitAction = {
                id: "Apply Script Changes",
                label: "Apply Script Changes",
                contextMenuOrder: 0,
                contextMenuGroupId: "script",
                keybindings: [
                    monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter,
                    monaco.KeyMod.CtrlCmd | monaco.KeyCode.Key_S,
                ],
                run: () => {
                    backend.updateScriptContent(this._editor.getValue());
                    this._updateEditCount(0);
                    this.setStatus("Applied script changes.");
                },
            }
            this._editor.addAction(commitAction);

            // Override global Control + S as commit shortcut 
            document.addEventListener('keydown', e => {
                if (e.ctrlKey && e.key === 's') {
                    // Prevent the Save dialog to open
                    e.preventDefault();
                    // Apply 
                    commitAction.run();
                }
            });

            // Handle Ctrl+Shift+Space
            const playStopAction = {
                id: "Start/Stop Playback",
                label: "Start/Stop Playback",
                contextMenuOrder: 1,
                contextMenuGroupId: "script",
                keybindings: [
                    monaco.KeyMod.CtrlCmd | monaco.KeyMod.Shift | monaco.KeyCode.Space,
                ],
                run: () => {
                    const playButton = document.getElementById('playButton');
                    if (!playButton.disabled) {
                        if (backend.isPlaying()) {
                            backend.stopPlaying();
                            playButton.style.color = null;
                        }
                        else {
                            backend.startPlaying();
                            playButton.style.color = 'var(--color-accent)';
                        }
                    }
                },
            }
            this._editor.addAction(playStopAction);

            // Stop all notes when leaving the page 
            document.addEventListener('visibilitychange', e => {
                if (document.visibilityState === 'hidden') {
                    backend.stopPlayingNotes();
                }
            });

            window.addEventListener("hashchange", () => {
                if (this._changedHashFromUserEdit) {
                    this._changedHashFromUserEdit = false;
                    return;
                }
                const script = this._decodeScriptFromHash() || "";
                this._changedScriptFromHash = true;
                this._editor.setValue(script);
                this._updateScript(script);
            });
            
            /*
            // TODO: Register a simple autocomplete provider for Lua for `pattern`
            monaco.languages.registerCompletionItemProvider('lua', {
                provideCompletionItems: function (model, position) {
                    const lineContent = model.getLineContent(position.lineNumber);
                    const textUntilPosition = model.getValueInRange({
                        startLineNumber: 1,
                        startColumn: 1,
                        endLineNumber: position.lineNumber,
                        endColumn: position.column
                    });
    
                    let insidePatternTable = false;
                    let braceDepth = 0;
                    let inPattern = false;
                    for (let i = 0; i < textUntilPosition.length; i++) {
                        const char = textUntilPosition[i];
    
                        if (textUntilPosition.substr(i, 6) === 'pattern') {
                            // Look ahead for opening brace
                            for (let j = i + 6; j < textUntilPosition.length; j++) {
                                if (textUntilPosition[j] === '{') {
                                    inPattern = true;
                                    braceDepth = 1;
                                    i = j;
                                    break;
                                } else if (textUntilPosition[j] !== ' ' && textUntilPosition[j] !== '\t' && textUntilPosition[j] !== '\n') {
                                    break;
                                }
                            }
                        } else if (inPattern) {
                            if (char === '{') {
                                braceDepth++;
                            } else if (char === '}') {
                                braceDepth--;
                                if (braceDepth === 0) {
                                    inPattern = false;
                                }
                            }
                        }
                    }

                    insidePatternTable = inPattern && braceDepth > 0;
                    if (insidePatternTable) {
                        const word = model.getWordUntilPosition(position);
                        const range = {
                            startLineNumber: position.lineNumber,
                            endLineNumber: position.lineNumber,
                            startColumn: word.startColumn,
                            endColumn: word.endColumn
                        };
                        return {
                            suggestions: [
                                {
                                    label: 'event',
                                    kind: monaco.languages.CompletionItemKind.Property,
                                    insertText: 'event = ',
                                    range: range,
                                    sortText: '1'
                                },
                                {
                                    label: 'pulse',
                                    kind: monaco.languages.CompletionItemKind.Property,
                                    insertText: 'pulse = ',
                                    range: range,
                                    sortText: '2'
                                }
                            ]
                        };
                    }
    
                    return { suggestions: [] };
                }
            });
            */
        });
    },

    // install script error change handler
    _initScriptErrorHandler: function () {
        window.on_script_error_changed = () => {
            this._updateScriptErrorsUI();
        }
    },

    // install script parameter change handler
    _initScriptParameterHandler: function () {
        window.on_script_parameters_changed = () => {
            this._updateParametersUI();
        }
        this._updateParametersUI();
    },

    // update script error display in editor and error panel
    _updateScriptErrorsUI: function () {
        const errorPane = document.getElementById('editor-error');
        console.assert(errorPane);

        const errorContent = document.getElementById('editor-error-content');
        console.assert(errorContent);

        const err = backend.getScriptError();
        // Clear previous markers
        if (this._editor) {
            monaco.editor.setModelMarkers(
                this._editor.getModel(),
                'owner',
                []
            );
        }
        if (err) {
            // Parse error and add to editor
            errorContent.textContent = err;
            errorPane.style.display = 'flex';
            const parsedError = this._parseLuaError(err);
            if (parsedError && this._editor) {
                monaco.editor.setModelMarkers(
                    this._editor.getModel(),
                    'owner',
                    [{
                        severity: monaco.MarkerSeverity.Error,
                        message: parsedError.message,
                        startLineNumber: parsedError.lineNumber,
                        startColumn: 1,
                        endLineNumber: parsedError.lineNumber,
                        endColumn: 100 // arbitrary large column
                    }]
                );
            }
        } else {
            // Clear error display
            errorContent.textContent = '';
            errorPane.style.display = 'none';
        }
    },

    // Show hide the "X edits" text
    _updateEditCount: function (count) {
        this._editCount = count;

        const editorStatusContent = document.getElementById('editor-status-content');
        const editCountSpan = document.getElementById('editCount');
        console.assert(editorStatusContent && editCountSpan);

        if (count > 0) {
            editCountSpan.textContent = `${count} edit${count === 1 ? '' : 's'}`;
            editorStatusContent.classList.remove('hidden');
            editorStatusContent.style.backgroundColor = 'var(--color-grid)';
        }
        else {
            editorStatusContent.classList.add('hidden');
            editorStatusContent.style.backgroundColor = 'unset'
        }
    },

    // helper function to get line info from Lua errors
    _parseLuaError: function (error) {
        // Parse Lua error format like: [string "buffer"]:3: 'then' expected near '='
        const match = error.match(/\[string ".*"\]:(\d+):\s*(.*)/);
        if (match) {
            return {
                lineNumber: parseInt(match[1]),
                message: match[2]
            };
        }
        return null;
    },

    // rebuild parameter controls
    _updateParametersUI: function () {
        const container = document.getElementById('parameters-container');
        console.assert(container);
        container.innerHTML = '';

        const parameters = backend.getScriptParameters();
        if (!parameters || parameters.length === 0) {
            container.style.display = 'none';
            return;
        }

        container.style.display = 'flex';

        parameters.forEach(param => {
            const controlWrapper = document.createElement('div');
            controlWrapper.className = 'parameter-control';

            const label = document.createElement('label');
            label.textContent = param.name + ":";
            label.title = param.description || param.name;
            controlWrapper.appendChild(label);

            let control;

            switch (param.type) {
                case 'boolean':
                    control = document.createElement('input');
                    control.type = 'checkbox';
                    control.checked = param.value;
                    control.addEventListener('change', (e) => {
                        backend.updateParameterValue(param.id, e.target.checked ? 1 : 0);
                    });
                    break;

                case 'integer':
                case 'float':
                    control = document.createElement('input');
                    control.type = 'number';

                    control.min = param.range.start;
                    control.max = param.range.end;

                    if (param.type === 'float') {
                        const step = (param.range.end - param.range.start) / 100;
                        if (step > 0) {
                            control.step = step;
                        }
                    } else {
                        control.step = 1;
                    }

                    control.value = param.value;
                    control.title = param.description;
                    control.addEventListener('change', (e) => {
                        let value = parseFloat(e.target.value);
                        if (isNaN(value)) {
                            // On invalid input, revert to the last known value from the backend.
                            e.target.value = param.value;
                            return;
                        }
                        // Clamp value to the defined range
                        const clampedValue = Math.max(param.range.start, Math.min(value, param.range.end));
                        // Update the input field to show the (potentially clamped) value
                        if (value !== clampedValue) {
                            e.target.value = clampedValue;
                        }
                        // Send the valid, clamped value to the backend
                        backend.updateParameterValue(param.id, clampedValue);
                    });
                    break;

                case 'enum':
                    control = document.createElement('select');
                    param.value_strings.forEach((name, index) => {
                        const option = document.createElement('option');
                        option.value = index;
                        option.textContent = name;
                        control.appendChild(option);
                    });
                    control.value = param.value;
                    control.addEventListener('change', (e) => {
                        backend.updateParameterValue(param.id, parseInt(e.target.value, 10));
                    });
                    break;
            }

            if (control) {
                controlWrapper.appendChild(control);
            }
            container.appendChild(controlWrapper);
        });
    },
}

// -------------------------------------------------------------------------------------------------

const webAssemblySupported = (() => {
    try {
        if (typeof WebAssembly === "object" && typeof WebAssembly.instantiate === "function") {
            const module = new WebAssembly.Module(
                Uint8Array.of(0x0, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00));
            if (module instanceof WebAssembly.Module) {
                return new WebAssembly.Instance(module) instanceof WebAssembly.Instance;
            }
        }
    }
    catch (e) {
        // ignore
    }
    return false;
})();

if (webAssemblySupported) {
    let Module = {
        print: (...args) => {
            let isError = false;
            app.setStatus(args.join(' '), isError)
        },
        printErr: (...args) => {
            let isError = true;
            app.setStatus(args.join(' '), isError)
        }
    }

    createModule(Module)
        .then((module) => {
            // initialize backend
            let err = backend.initialize(module);
            if (err) {
                const isError = true;
                app.setStatus(err, true);
            }
            else {
                // initialize app
                app.initialize();
                app.setStatus("Ready");
            }
        }).catch((err) => {
            let isError = true;
            app.setStatus(err.message || "WASM failed to load", isError);
        });

    // redirect global errors
    window.addEventListener("unhandledrejection", function (event) {
        let isError = true;
        app.setStatus(event.reason, isError);
    });
    window.onerror = (message, filename, lineno, colno, error) => {
        let isError = true;
        app.setStatus(message || "Unknown window error", isError);
    };

}
else {
    const isError = true;
    app.setStatus("This page requires WebAssembly support, " +
        "which appears to be unavailable in this browser.", isError);

    let spinner = document.getElementById('spinner');
    if (spinner) {
        spinner.style.display = "None";
    }
}
