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
  { "c#4", "f4", "c5", "f5", "g#4", "f4" },
  { "a#4", "g#4", "f4", "a#3", "g#3", "f3" },
  { "f4", "c#4", "f5", "g#4", "c5", "g#4" },
  { "g#4", "a#3", "f4", "a#3", "f3", "g#3" },
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
    _backend: undefined,
    _isPlaying: false,
    _currentInstrument: null,

    initialize: function (backend) {
        this._backend = backend;

        const err = this._backend.ccall('initialize_app', 'string', [])
        if (err) {
            return err
        }

        this.setBpm(defaultBpm);
        this.setInstrument(defaultInstrument);
        this.updateScript(defaultScriptContent);

        return undefined;
    },

    isPlaying: function () {
        return this._isPlaying;
    },

    startPlaying: function () {
        this._backend.ccall("start_playing");
        this._isPlaying = true;
    },

    stopPlaying: function () {
        this._backend.ccall("stop_playing");
        this._isPlaying = false;
    },

    stopPlayingNotes: function () {
        this._backend.ccall("stop_playing_notes");
    },

    sendMidiNoteOn: function (note, velocity) {
        this._backend.ccall("midi_note_on", 'undefined', ['number', 'number'], [note, velocity]);
    },

    sendMidiNoteOff: function (note) {
        this._backend.ccall("midi_note_off", 'undefined', ['number'], [note]);
    },

    setVolume: function (volume) {
        this._backend.ccall("set_volume", "undefined", ["number"], [volume]);
    },

    setBpm: function (bpm) {
        this._backend.ccall("set_bpm", 'undefined', ['number'], [bpm]);
    },

    getInstrument: function () {
        return this._currentInstrument;
    },

    setInstrument: function (instrument) {
        this._currentInstrument = instrument;
        this._backend.ccall("set_instrument", 'undefined', ['number'], [instrument]);
    },

    getQuickstartScripts: function () {
        const stringPtr = this._backend.ccall('quickstart_scripts', 'number', [])
        const examplesJson = this._backend.UTF8ToString(stringPtr);
        this._freeCString(stringPtr)
        return JSON.parse(examplesJson);
    },

    getExampleScripts: function () {
        const stringPtr = this._backend.ccall('example_scripts', 'number', [])
        const examplesJson = this._backend.UTF8ToString(stringPtr);
        this._freeCString(stringPtr)
        return JSON.parse(examplesJson);
    },

    updateScript: function (content) {
        this._backend.ccall("update_script", 'undefined', ['string'], [content]);
    },

    getScriptError: function () {
        let stringPtr = this._backend.ccall('script_error', 'number', [])
        const error = this._backend.UTF8ToString(stringPtr);
        this._freeCString(stringPtr)
        return error;
    },

    getScriptParameters: function () {
        let stringPtr = this._backend.ccall('script_parameters', 'number', [])
        const json = this._backend.UTF8ToString(stringPtr);
        const parameters = JSON.parse(json);
        this._freeCString(stringPtr)
        return parameters;
    },

    setScriptParameterValue: function (id, value) {
        this._backend.ccall("set_script_parameter_value", "undefined", ["string", "number"], [id, value]);
    },


    getSamples: function () {
        const stringPtr = this._backend.ccall('samples', 'number', [])
        const samplesJson = this._backend.UTF8ToString(stringPtr);
        this._freeCString(stringPtr)
        return JSON.parse(samplesJson);
    },

    loadSample: function (filename, buffer) {
        const data = new Uint8Array(buffer);
        const newSampleId = this._backend.ccall(
            'load_sample',
            'number',
            ['string', 'array', 'number'],
            [filename, data, data.length]
        );
        return newSampleId;
    },

    clearSamples: function () {
        this._backend.ccall('clear_samples', 'undefined', []);
    },

    getMixers: function () {
        const stringPtr = this._backend.ccall('mixers', 'number', []);
        const json = this._backend.UTF8ToString(stringPtr);
        this._freeCString(stringPtr);
        return JSON.parse(json);
    },

    removeMixer: function (mixerId) {
        return this._backend.ccall('remove_mixer', 'number', ['number'], [mixerId]);
    },

    getAvailableEffects: function () {
        const stringPtr = this._backend.ccall('available_effects', 'number', []);
        const json = this._backend.UTF8ToString(stringPtr);
        this._freeCString(stringPtr);
        return JSON.parse(json);
    },

    addEffectToMixer: function (mixerId, effectName) {
        const stringPtr = this._backend.ccall('add_effect_to_mixer', 'number', ['number', 'string'], [mixerId, effectName]);
        if (stringPtr !== 0) {
            const json = this._backend.UTF8ToString(stringPtr);
            const result = JSON.parse(json);
            this._freeCString(stringPtr);
            return result;
        }
        return null;
    },

    removeEffectFromMixer: function (effectId) {
        return this._backend.ccall('remove_effect_from_mixer', 'number', ['number'], [effectId]);
    },

    moveEffectInMixer: function (effectId, mixerId, direction) {
        return this._backend.ccall('move_effect_in_mixer', 'number', ['number', 'number', 'number'],
            [effectId, mixerId, direction]);
    },

    getEffectParameterString: function (effectId, paramId, normalizedValue) {
        const stringPtr = this._backend.ccall('effect_parameter_string', 'number', ['number', 'number', 'number'],
            [effectId, paramId, normalizedValue]);
        if (stringPtr !== 0) {
            const valueStr = this._backend.UTF8ToString(stringPtr);
            this._freeCString(stringPtr);
            return valueStr;
        }
        return null;
    },

    setEffectParameterValue: function (effectId, paramId, normalizedValue) {
        this._backend.ccall('set_effect_parameter_value', 'number', ['number', 'number', 'number'],
            [effectId, paramId, normalizedValue]);
    },

    _freeCString: function (stringPtr) {
        this._backend.ccall('free_cstring', 'undefined', ['number'], [stringPtr])
    },
};

// -------------------------------------------------------------------------------------------------

const app = {
    _initialized: false,
    _editor: undefined,
    _editCount: 0,
    _changedHashFromUserEdit: false,
    _changedScriptFromHash: false,
    _midiEnabled: false,
    _fxManager: undefined,

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
        this._fxManager = new FxManager();
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

    _togglePlayButton: function (isPlaying) {
        const playButton = document.getElementById('playButton');
        const i = playButton.querySelector('i');
        if (isPlaying) {
            i.classList.replace('fa-play', 'fa-stop');
            playButton.classList.add('enabled');
        } else {
            i.classList.replace('fa-stop', 'fa-play');
            playButton.classList.remove('enabled');
        }
    },

    _togglePlayback: function () {
        const playButton = document.getElementById('playButton');
        if (!playButton.disabled) {
            if (backend.isPlaying()) {
                backend.stopPlaying();
                this.setStatus("Playback stopped.");
            } else {
                backend.startPlaying();
                this.setStatus("Playing...");
            }
            this._togglePlayButton(backend.isPlaying());
        }
    },

    // Init transport controls
    _initControls: function () {
        // Set up control handlers
        const playButton = document.getElementById('playButton');
        const midiButton = document.getElementById('midiButton');
        console.assert(playButton && midiButton);

        playButton.addEventListener('click', () => this._togglePlayback());
        playButton.title = "Toggle Playback (Ctrl+Shift+Space)";

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
                backend.setBpm(clampedBpm);
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
        let currentMidiNotes = new Set();

        const handleMidiMessage = (message) => {
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

        const enableMidi = () => {
            if (!navigator.requestMIDIAccess) {
                return Promise.reject(new Error("Web MIDI API not supported"));
            }
            return navigator.requestMIDIAccess()
                .then(access => {
                    midiAccess = access;
                    this._midiEnabled = true;
                    midiButton.classList.add("enabled");
                    // Start listening to MIDI input
                    for (let input of midiAccess.inputs.values()) {
                        input.onmidimessage = handleMidiMessage;
                    }
                    // stop regular playback
                    if (backend.isPlaying()) {
                        backend.stopPlaying();
                    }
                    app.setStatus("MIDI input enabled. Press one or more notes on your keyboard to play the script...");
                });
        }

        const disableMidi = () => {
            this._midiEnabled = false;
            midiButton.classList.remove("enabled");
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


        midiButton.addEventListener('click', () => {
            if (!this._midiEnabled) {
                enableMidi().then(() => {
                    // Disable play/stop buttons on success
                    this._togglePlayButton(false);
                    playButton.disabled = true;
                }).catch(err => {
                    const isError = true;
                    app.setStatus("Failed to access MIDI: " + err, isError);
                });
            } else {
                disableMidi().then(() => {
                    // Re-enable play/stop buttons
                    playButton.disabled = false;
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
                    this._selectInstrument(newId)
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

    _selectInstrument: function (id) {
        if (id === null || id === undefined) return;
        const select = document.getElementById('sampleSelect');
        const numericId = Number(id);
        const value = Math.max(0, Math.min(numericId, select.options.length - 1));
        select.value = numericId;

        backend.setInstrument(numericId);
        this.setStatus(`Set default instrument: '${select.options[value].innerHTML}'`);
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
                this._selectInstrument(event.target.value);
                this._updateHash();
            };

            // set last sample as default instrument
            this._selectInstrument(samples[samples.length - 1].id)
        } else {
            const option = document.createElement('option');
            option.value = 'none';
            option.textContent = 'No samples loaded';
            select.appendChild(option);
            select.onchange = null;
            backend.setInstrument(-1);
        }
    },

    _updateScript: function ({ script, name, instrument }) {
        this._selectInstrument(instrument);

        if (backend.isPlaying()) {
            backend.stopPlaying();
            backend.updateScript(script);
            backend.startPlaying();
        } else {
            backend.updateScript(script);
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

        const appendExampleLink = (example) => {
            const li = document.createElement('li');
            const a = document.createElement('a');
            a.textContent = example.name;
            a.classList.add("example-link")
            li.appendChild(a);
            examplesList.appendChild(li);
            a.onclick = () => {
                window.location.hash = `#${this._encodeScript({
                    script: example.content,
                    name: example.name,
                    instrument: document.getElementById("sampleSelect").value
                })}`;
                document.querySelectorAll(".example-link").forEach(link => {
                    link.classList.remove("selected")
                });
                a.classList.add("selected");
            }
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

    _encodeScript: function ({ script, name, instrument }) {
        return btoa(JSON.stringify({ script, name, instrument }));
    },

    _decodeScriptFromHash: function (defaultScriptData = { script: "", name: "untitled", instrument: null }) {
        const hash = window.location.hash;
        if (hash.length < 2) {
            return defaultScriptData;
        }
        try {
            const string = atob(hash.substring(1).split('?')[0]);
            const object = JSON.parse(string)
            return object;
        } catch (e) {
            return defaultScriptData;
        }
    },

    _updateHash: function () {
        this._changedHashFromUserEdit = true;
        window.location.hash = this._encodeScript({
            script: this._editor.getValue(),
            name: "custom",
            instrument: document.getElementById('sampleSelect').value
        });
    },

    // Initialize Monaco editor
    _initEditor: function () {
        require.config({ paths: { 'vs': 'https://cdnjs.cloudflare.com/ajax/libs/monaco-editor/0.52.2/min/vs' } });

        let editorElement = document.getElementById('editor');
        console.assert(editorElement);


        require(['vs/editor/editor.main'], () => {
            // Try parsing script from URL hash or use the default
            const scriptData = this._decodeScriptFromHash({
                script: defaultScriptContent,
                name: "Default Script",
            });

            // Create editor
            this._editor = monaco.editor.create(editorElement, {
                value: scriptData.script,
                language: 'lua',
                theme: 'vs-dark',
                minimap: { enabled: false },
                scrollBeyondLastLine: false,
                automaticLayout: true,
                wordWrap: 'on',
                acceptSuggestionOnCommitCharacter: true
            });

            this._updateScript(scriptData);

            // Track edits
            this._editor.onDidChangeModelContent(() => {
                if (this._changedScriptFromHash) {
                    this._changedScriptFromHash = false;
                    return;
                }
                this._updateHash();
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
                    monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS,
                ],
                run: () => {
                    backend.updateScript(this._editor.getValue());
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
                    if (this._midiEnabled) return;
                    this._togglePlayback()
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
                const scriptData = this._decodeScriptFromHash();
                this._changedScriptFromHash = true;
                this._editor.setValue(scriptData.script);
                this._updateScript(scriptData);
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
                        backend.setScriptParameterValue(param.id, e.target.checked ? 1 : 0);
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
                        backend.setScriptParameterValue(param.id, clampedValue);
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
                        backend.setScriptParameterValue(param.id, parseInt(e.target.value, 10));
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

// FX Manager Class
class FxManager {
    constructor() {
        this.mixers = new Map();
        this.availableEffects = [];
        this.initUI();
    }

    getMixerIdForInstrument(instrumentId) {
        for (const [mixerId, mixer] of this.mixers.entries()) {
            if (mixer.instrument_id === instrumentId) {
                return mixerId;
            }
        }
        return null;
    }

    initUI() {
        this.availableEffects = backend.getAvailableEffects();

        const fxEditorButton = document.getElementById('fxEditorButton');
        const fxChainContainer = document.getElementById('fxChainContainer');
        const sampleSelect = document.getElementById('sampleSelect');

        fxEditorButton.addEventListener('click', () => {
            const isVisible = fxChainContainer.classList.contains('visible');
            if (isVisible) {
                fxChainContainer.classList.remove('visible');
                fxEditorButton.classList.remove('enabled');
            } else {
                this.refreshMixers();
                fxChainContainer.classList.add('visible');
                fxEditorButton.classList.add('enabled');

                const currentInstrumentId = backend.getInstrument();
                if (currentInstrumentId !== null && currentInstrumentId >= 0) {
                    this.onInstrumentChanged(currentInstrumentId);
                }
            }
        });

        // Listen to main sample selector changes
        sampleSelect.addEventListener('change', (e) => {
            const instrumentId = parseInt(e.target.value);
            if (!isNaN(instrumentId) && fxChainContainer.classList.contains('visible')) {
                this.refreshMixers();
                this.onInstrumentChanged(instrumentId);
            }
        });
    }

    refreshMixers() {
        const mixers = backend.getMixers();
        this.mixers.clear();
        mixers.forEach(mixer => {
            this.mixers.set(mixer.id, mixer);
        });
        this.updateAddEffectButtons();
    }

    onInstrumentChanged(instrumentId) {
        const fxChainContainer = document.getElementById('fxChainContainer');
        if (!fxChainContainer.classList.contains('visible')) {
            return;
        }
        const mixerId = this.getMixerIdForInstrument(instrumentId);
        this.updateChainUI(mixerId);
        this.updateAddEffectButtons();
    }

    updateAddEffectButtons() {
        const addEffectMenu = document.getElementById('fxAddEffectMenu');
        addEffectMenu.innerHTML = '';

        this.availableEffects.forEach(effectName => {
            const button = document.createElement('button');
            button.textContent = `+ ${effectName}`;
            button.addEventListener('click', () => this.addEffect(effectName));
            addEffectMenu.appendChild(button);
        });
    }

    updateChainUI(mixerId) {
        const devicesContainer = document.getElementById('fxChainDevices');
        devicesContainer.innerHTML = '';

        if (mixerId === null || isNaN(mixerId) || mixerId < 0) {
            const empty = document.createElement('div');
            empty.className = 'fx-chain-empty';
            empty.textContent = 'No sample selected.';
            devicesContainer.appendChild(empty);
            return;
        }

        const mixer = this.mixers.get(mixerId);
        if (!mixer || mixer.effects.length === 0) {
            const empty = document.createElement('div');
            empty.className = 'fx-chain-empty';
            empty.textContent = 'No effects. Add an effect using the buttons above.';
            devicesContainer.appendChild(empty);
            return;
        }

        mixer.effects.forEach(effect => {
            const card = this.createDeviceCard(effect, mixerId);
            devicesContainer.appendChild(card);
        });

        // Handle device drops
        devicesContainer.addEventListener('dragover', (e) => {
            e.preventDefault();
            e.stopPropagation();
            e.dataTransfer.dropEffect = 'move';

            const draggingCard = document.querySelector('.dragging');
            if (!draggingCard) return;

            // Remove existing markers
            document.querySelectorAll('.fx-drop-marker').forEach(m => m.remove());

            // Find the best position to insert the marker
            const cards = Array.from(devicesContainer.querySelectorAll('.fx-device-card:not(.dragging)'));

            if (cards.length === 0) {
                // No other cards, just add marker at the end
                const marker = document.createElement('div');
                marker.className = 'fx-drop-marker';
                marker.dataset.dropPosition = 'end';
                devicesContainer.appendChild(marker);
            } else {
                // Find which card we're closest to
                let closestCard = null;
                let closestDistance = Infinity;
                let insertBefore = true;

                cards.forEach(card => {
                    const rect = card.getBoundingClientRect();
                    const cardCenter = rect.left + rect.width / 2;
                    const distance = Math.abs(e.clientX - cardCenter);
                    if (distance < closestDistance) {
                        closestDistance = distance;
                        closestCard = card;
                        insertBefore = e.clientX < cardCenter;
                    }
                });

                if (closestCard) {
                    const marker = document.createElement('div');
                    marker.className = 'fx-drop-marker';
                    if (insertBefore) {
                        closestCard.parentNode.insertBefore(marker, closestCard);
                        marker.dataset.dropPosition = 'before';
                        marker.dataset.targetEffectId = closestCard.dataset.effectId;
                    } else {
                        closestCard.parentNode.insertBefore(marker, closestCard.nextSibling);
                        marker.dataset.dropPosition = 'after';
                        marker.dataset.targetEffectId = closestCard.dataset.effectId;
                    }
                }
            }
        });

        devicesContainer.addEventListener('drop', (e) => {
            e.preventDefault();

            const marker = devicesContainer.querySelector('.fx-drop-marker');
            if (!marker) return;

            const draggedEffectId = parseInt(e.dataTransfer.getData('text/plain'));
            const dropPosition = marker.dataset.dropPosition;
            const targetEffectId = marker.dataset.targetEffectId;

            // Clean up
            document.querySelectorAll('.fx-drop-marker').forEach(m => m.remove());

            if (dropPosition === 'end') {
                // Drop at the end - move to last position
                const mixer = this.mixers.get(mixerId);
                if (!mixer) return;

                const draggedIndex = mixer.effects.findIndex(e => e.id === draggedEffectId);
                if (draggedIndex === -1) return;

                const lastIndex = mixer.effects.length - 1;
                if (draggedIndex === lastIndex) return; // Already at the end

                const direction = lastIndex - draggedIndex;
                const result = backend.moveEffectInMixer(draggedEffectId, mixerId, direction);
                if (result === 0) {
                    app.setStatus(`Moved effect to end`);
                    this.refreshMixers();
                    this.onInstrumentChanged(backend.getInstrument());
                }
            } else if (targetEffectId) {
                // Drop relative to another card
                this.reorderEffect(draggedEffectId, parseInt(targetEffectId), dropPosition, mixerId);
            }
        });

        devicesContainer.addEventListener('dragleave', (e) => {
            // Only remove markers if we're leaving the container entirely
            const rect = devicesContainer.getBoundingClientRect();
            if (e.clientX < rect.left || e.clientX > rect.right ||
                e.clientY < rect.top || e.clientY > rect.bottom) {
                document.querySelectorAll('.fx-drop-marker').forEach(m => m.remove());
            }
        });
    }

    createDeviceCard(effect, mixerId) {
        const card = document.createElement('div');
        card.className = 'fx-device-card';
        card.dataset.effectId = effect.id;
        card.dataset.mixerId = mixerId;

        const header = document.createElement('div');
        header.className = 'fx-device-header';
        header.draggable = true;

        // Drag start - attach to header
        header.addEventListener('dragstart', (e) => {
            e.dataTransfer.effectAllowed = 'move';
            e.dataTransfer.setData('text/plain', effect.id);
            card.classList.add('dragging');
        });

        // Drag end - attach to header
        header.addEventListener('dragend', (e) => {
            card.classList.remove('dragging');
            document.querySelectorAll('.fx-drop-marker').forEach(m => m.remove());
            document.querySelectorAll('.fx-device-card').forEach(c => {
                delete c.dataset.dropPosition;
            });
        });

        const name = document.createElement('div');
        name.className = 'fx-device-name';
        name.textContent = effect.name;

        const removeBtn = document.createElement('button');
        removeBtn.className = 'fx-device-remove';
        removeBtn.textContent = '×';
        removeBtn.title = 'Remove Effect';
        removeBtn.addEventListener('click', (e) => {
            e.stopPropagation();
            this.removeEffect(effect.id);
        });

        header.appendChild(name);
        header.appendChild(removeBtn);
        card.appendChild(header);

        // Add parameters inline
        if (effect.parameters.length > 0) {
            const paramsContainer = document.createElement('div');
            paramsContainer.className = 'fx-device-params';

            effect.parameters.forEach(param => {
                const paramControl = this.createInlineParameterControl(effect.id, param);
                paramsContainer.appendChild(paramControl);
            });

            card.appendChild(paramsContainer);
        }

        return card;
    }

    reorderEffect(draggedEffectId, targetEffectId, dropPosition, mixerId) {
        const mixer = this.mixers.get(mixerId);
        if (!mixer) return;

        // Find positions
        const draggedIndex = mixer.effects.findIndex(e => e.id === draggedEffectId);
        const targetIndex = mixer.effects.findIndex(e => e.id === targetEffectId);

        // something to drag?
        if (draggedIndex === -1 || targetIndex === -1) return;
        // dragging onto the right side of the device left to us 
        if (dropPosition === 'after' && targetIndex === draggedIndex - 1) return;
        // dragging onto the left side of the device right to us 
        if (dropPosition === 'before' && targetIndex === draggedIndex + 1) return;

        // Calculate direction to move
        const direction = targetIndex - draggedIndex;

        const result = backend.moveEffectInMixer(draggedEffectId, mixerId, direction);
        if (result === 0) {
            app.setStatus(`Reordered effect`);
            this.refreshMixers();
            const selectedInstrumentId = backend.getInstrument();
            if (selectedInstrumentId !== null && selectedInstrumentId >= 0) {
                this.onInstrumentChanged(selectedInstrumentId);
            }
        } else {
            const isError = true;
            app.setStatus(`Failed to reorder effect`, isError);
        }
    }

    createInlineParameterControl(effectId, param) {
        const container = document.createElement('div');
        container.className = 'fx-device-param';

        const label = document.createElement('label');
        label.className = 'fx-device-param-label';
        const nameSpan = document.createElement('span');
        nameSpan.textContent = param.name;
        const valueSpan = document.createElement('span');
        valueSpan.className = 'fx-device-param-value';

        label.appendChild(nameSpan);
        label.appendChild(valueSpan);

        const updateValueDisplay = (normalizedValue) => {
            const valueStr = backend.getEffectParameterString(effectId, param.id, normalizedValue);
            if (valueStr) {
                valueSpan.textContent = valueStr;
            }
        };

        let input;
        if (param.type === 'Float' || param.type === 'Integer') {
            input = document.createElement('input');
            input.type = 'range';
            input.min = '0';
            input.max = '1';
            input.step = '0.01';
            input.value = param.default;
            updateValueDisplay(param.default);

            input.addEventListener('input', (e) => {
                const normalized = parseFloat(e.target.value);
                backend.setEffectParameterValue(effectId, param.id, normalized);
                updateValueDisplay(normalized);
            });
        } else if (param.type === 'Boolean') {
            input = document.createElement('input');
            input.type = 'checkbox';
            input.checked = param.default > 0.5;
            updateValueDisplay(param.default);

            input.addEventListener('change', (e) => {
                const normalized = e.target.checked ? 1.0 : 0.0;
                backend.setEffectParameterValue(effectId, param.id, normalized);
                updateValueDisplay(normalized);
            });
        } else if (param.type === 'Enum') {
            input = document.createElement('select');
            const defaultIndex = Math.floor(param.default * (param.values.length - 1));
            updateValueDisplay(param.default);
            param.values.forEach((val, idx) => {
                const option = document.createElement('option');
                option.value = idx;
                option.textContent = val;
                if (idx === defaultIndex) {
                    option.selected = true;
                }
                input.appendChild(option);
            });

            input.addEventListener('change', (e) => {
                const idx = parseInt(e.target.value);
                const normalized = idx / (param.values.length - 1);
                backend.setEffectParameterValue(effectId, param.id, normalized);
                updateValueDisplay(normalized);
            });
        }

        container.appendChild(label);
        if (input) {
            container.appendChild(input);
        }

        return container;
    }

    addEffect(effectName) {
        const selectedInstrumentId = backend.getInstrument();

        if (selectedInstrumentId === null || selectedInstrumentId < 0) {
            const isError = true;
            app.setStatus('No sample selected', isError);
            return;
        }

        const mixerId = this.getMixerIdForInstrument(selectedInstrumentId);
        if (mixerId === null) {
            const isError = true;
            app.setStatus('No mixer found for selected sample', isError);
            return;
        }

        const result = backend.addEffectToMixer(mixerId, effectName);
        if (result) {
            app.setStatus(`Added ${effectName} effect`);
            this.refreshMixers();
            this.onInstrumentChanged(selectedInstrumentId);
        } else {
            const isError = true;
            app.setStatus(`Failed to add ${effectName} effect`, isError);
        }
    }

    removeEffect(effectId) {
        const result = backend.removeEffectFromMixer(effectId);
        if (result === 0) {
            app.setStatus(`Removed effect`);
            const selectedInstrumentId = backend.getInstrument();
            this.refreshMixers();
            if (selectedInstrumentId !== null && selectedInstrumentId >= 0) {
                this.onInstrumentChanged(selectedInstrumentId);
            }
        } else {
            const isError = true;
            app.setStatus(`Failed to remove effect`, isError);
        }
    }
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
