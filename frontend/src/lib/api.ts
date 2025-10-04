import type { Property } from './types';

const API_BASE_URL = 'http://localhost:3001/api';

export async function fetchProperties(): Promise<Property[]> {
	const response = await fetch(`${API_BASE_URL}/properties`);
	if (!response.ok) {
		throw new Error('Failed to fetch properties');
	}
	return response.json();
}

export async function fetchHealth(): Promise<{ message: string; status: string }> {
	const response = await fetch(`${API_BASE_URL}/health`);
	if (!response.ok) {
		throw new Error('Failed to fetch health status');
	}
	return response.json();
}