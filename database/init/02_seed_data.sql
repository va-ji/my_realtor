-- Seed data for Australian real estate properties
-- Realistic property data across major Australian cities

INSERT INTO properties (address, suburb, state, bedrooms, price, weekly_rent, latitude, longitude) VALUES
-- Melbourne, VIC
('12/45 Collins Street', 'Melbourne', 'VIC', 2, 650000, 550, -37.8136, 144.9631),
('8 Chapel Street', 'St Kilda', 'VIC', 1, 480000, 420, -37.8677, 144.9811),
('156 Brunswick Street', 'Fitzroy', 'VIC', 2, 720000, 580, -37.7984, 144.9789),
('23 Park Street', 'South Melbourne', 'VIC', 3, 890000, 680, -37.8315, 144.9566),

-- Sydney, NSW
('301/88 George Street', 'Sydney', 'NSW', 1, 750000, 650, -33.8688, 151.2093),
('45 Oxford Street', 'Darlinghurst', 'NSW', 2, 920000, 750, -33.8785, 151.2199),
('12 Beach Road', 'Bondi Beach', 'NSW', 2, 1150000, 850, -33.8915, 151.2767),
('78 Harris Street', 'Pyrmont', 'NSW', 1, 680000, 580, -33.8688, 151.1952),
('5 King Street', 'Newtown', 'NSW', 3, 1050000, 780, -33.8977, 151.1794),

-- Brisbane, QLD
('12/200 Adelaide Street', 'Brisbane City', 'QLD', 2, 520000, 480, -27.4698, 153.0251),
('34 Grey Street', 'South Brisbane', 'QLD', 1, 430000, 400, -27.4748, 153.0195),
('89 Boundary Street', 'West End', 'QLD', 2, 580000, 520, -27.4809, 153.0094),
('15 James Street', 'Fortitude Valley', 'QLD', 1, 390000, 380, -27.4573, 153.0349),

-- Perth, WA
('45 St Georges Terrace', 'Perth', 'WA', 2, 550000, 480, -31.9505, 115.8605),
('12 Marine Parade', 'Cottesloe', 'WA', 3, 980000, 720, -32.0024, 115.7572),

-- Adelaide, SA
('78 North Terrace', 'Adelaide', 'SA', 1, 380000, 350, -34.9207, 138.6011),
('23 Rundle Street', 'Kent Town', 'SA', 2, 490000, 420, -34.9238, 138.6167),

-- Canberra, ACT
('45 London Circuit', 'Canberra City', 'ACT', 2, 620000, 560, -35.2809, 149.1300),
('12 Constitution Avenue', 'Reid', 'ACT', 3, 780000, 650, -35.2820, 149.1362),

-- Hobart, TAS
('56 Elizabeth Street', 'Hobart', 'TAS', 2, 420000, 380, -42.8821, 147.3272),
('23 Sandy Bay Road', 'Sandy Bay', 'TAS', 3, 650000, 520, -42.8985, 147.3279);

-- Add some price history data for trend analysis
INSERT INTO price_history (property_id, price, weekly_rent, recorded_date) VALUES
-- Melbourne properties
(1, 620000, 520, '2024-01-15'),
(1, 635000, 535, '2024-06-15'),
(1, 650000, 550, '2024-12-15'),

(2, 460000, 400, '2024-01-15'),
(2, 470000, 410, '2024-06-15'),
(2, 480000, 420, '2024-12-15'),

-- Sydney properties
(5, 700000, 600, '2024-01-15'),
(5, 725000, 625, '2024-06-15'),
(5, 750000, 650, '2024-12-15'),

(7, 1050000, 780, '2024-01-15'),
(7, 1100000, 815, '2024-06-15'),
(7, 1150000, 850, '2024-12-15'),

-- Brisbane properties
(10, 480000, 450, '2024-01-15'),
(10, 500000, 465, '2024-06-15'),
(10, 520000, 480, '2024-12-15');