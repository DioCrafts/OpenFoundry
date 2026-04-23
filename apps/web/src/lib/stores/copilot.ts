import { writable } from 'svelte/store';

interface CopilotState {
	open: boolean;
	seedQuestion: string;
}

function createCopilotStore() {
	const { subscribe, set, update } = writable<CopilotState>({
		open: false,
		seedQuestion: '',
	});

	return {
		subscribe,
		open(seedQuestion = '') {
			set({ open: true, seedQuestion });
		},
		close() {
			update((state) => ({ ...state, open: false, seedQuestion: '' }));
		},
		toggle(seedQuestion = '') {
			update((state) => ({
				open: !state.open,
				seedQuestion: seedQuestion || state.seedQuestion,
			}));
		},
		setQuestion(seedQuestion: string) {
			update((state) => ({ ...state, seedQuestion }));
		},
	};
}

export const copilot = createCopilotStore();