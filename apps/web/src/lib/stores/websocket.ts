import { writable } from 'svelte/store';

import { issueNotificationSocketTicket, type NotificationSocketEvent } from '$lib/api/notifications';

function notificationSocketUrl(ticket: string) {
	const configured = import.meta.env.PUBLIC_NOTIFICATION_WS_URL as string | undefined;
	if (configured) {
		const url = new URL(configured);
		url.searchParams.set('ticket', ticket);
		return url.toString();
	}

	if (typeof window === 'undefined') {
		return '';
	}

	const protocol = window.location.protocol === 'https:' ? 'wss' : 'ws';
	const url = new URL(`${protocol}://${window.location.hostname}:50069/api/v1/notifications/ws`);
	url.searchParams.set('ticket', ticket);
	return url.toString();
}

function createNotificationWebsocketStore() {
	const connected = writable(false);
	let socket: WebSocket | null = null;
	let requestSerial = 0;

	async function connect(token: string, onMessage: (event: NotificationSocketEvent) => void) {
		if (!token || typeof window === 'undefined') {
			return;
		}

		const serial = ++requestSerial;
		let ticket = '';
		try {
			const response = await issueNotificationSocketTicket();
			ticket = response.ticket;
		} catch {
			connected.set(false);
			return;
		}

		if (serial !== requestSerial) {
			return;
		}

		const nextUrl = notificationSocketUrl(ticket);
		if (!nextUrl) {
			return;
		}

		if (socket && socket.url === nextUrl && (socket.readyState === WebSocket.OPEN || socket.readyState === WebSocket.CONNECTING)) {
			return;
		}

		disconnect();
		socket = new WebSocket(nextUrl);
		socket.onopen = () => connected.set(true);
		socket.onclose = () => {
			connected.set(false);
			socket = null;
		};
		socket.onerror = () => connected.set(false);
		socket.onmessage = (message) => {
			try {
				onMessage(JSON.parse(String(message.data)) as NotificationSocketEvent);
			} catch {
				// Ignore malformed frames.
			}
		};
	}

	function disconnect() {
		requestSerial += 1;
		if (socket) {
			socket.close();
			socket = null;
		}
		connected.set(false);
	}

	return {
		connected,
		connect,
		disconnect,
	};
}

export const notificationWebsocket = createNotificationWebsocketStore();
