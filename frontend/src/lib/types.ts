export interface Property {
	id: number;
	address: string;
	suburb: string;
	state: string;
	bedrooms: number | null;
	price: number | null;
	weekly_rent: number | null;
	latitude: string | null;
	longitude: string | null;
	rental_yield: number | null;
}

export interface ApiResponse<T> {
	data?: T;
	error?: string;
}