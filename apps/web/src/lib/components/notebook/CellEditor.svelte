<script lang="ts">
	import { onMount } from 'svelte';
	import type * as Monaco from 'monaco-editor';

	interface Props {
		value: string;
		language: string;
		minHeight?: number;
		onChange?: (value: string) => void;
		onBlur?: (value: string) => void;
	}

	let {
		value,
		language,
		minHeight = 160,
		onChange,
		onBlur,
	}: Props = $props();

	let container = $state<HTMLDivElement | null>(null);
	let monaco = $state<typeof import('monaco-editor') | null>(null);
	let editor = $state<Monaco.editor.IStandaloneCodeEditor | null>(null);
	let syncing = false;

	onMount(() => {
		let changeSubscription: Monaco.IDisposable | null = null;
		let blurSubscription: Monaco.IDisposable | null = null;
		let disposed = false;

		async function initializeEditor() {
			monaco = await import('monaco-editor');

			if (disposed || !container) {
				return;
			}

			const createdEditor = monaco.editor.create(container, {
				value,
				language,
				automaticLayout: true,
				minimap: { enabled: false },
				fontSize: 13,
				lineNumbers: 'on',
				roundedSelection: false,
				scrollBeyondLastLine: false,
				wordWrap: language === 'markdown' ? 'on' : 'off',
				theme: document.documentElement.classList.contains('dark') ? 'vs-dark' : 'vs',
			});

			if (disposed) {
				createdEditor.dispose();
				return;
			}

			editor = createdEditor;

			changeSubscription = createdEditor.onDidChangeModelContent(() => {
				if (syncing) {
					return;
				}
				onChange?.(createdEditor.getValue());
			});

			blurSubscription = createdEditor.onDidBlurEditorText(() => {
				onBlur?.(createdEditor.getValue());
			});
		}

		void initializeEditor();

		return () => {
			disposed = true;
			changeSubscription?.dispose();
			blurSubscription?.dispose();
			editor?.dispose();
			editor = null;
		};
	});

	$effect(() => {
		if (!editor) {
			return;
		}
		if (editor.getValue() === value) {
			return;
		}

		syncing = true;
		editor.setValue(value);
		syncing = false;
	});

	$effect(() => {
		if (!editor || !monaco) {
			return;
		}

		const model = editor.getModel();
		if (model) {
			monaco.editor.setModelLanguage(model, language);
		}

		editor.updateOptions({
			wordWrap: language === 'markdown' ? 'on' : 'off',
		});
	});
</script>

<div
	bind:this={container}
	class="w-full"
	style={`height: ${Math.max(minHeight, 96)}px;`}
></div>
