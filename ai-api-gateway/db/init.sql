CREATE TABLE IF NOT EXISTS providers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    url TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS models (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS provider_models (
    provider_id TEXT NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
    model_id TEXT NOT NULL REFERENCES models(id) ON DELETE CASCADE,
    PRIMARY KEY (provider_id, model_id)
);

INSERT INTO providers (id, name, url) VALUES
    ('openai', 'OpenAI', 'https://api.openai.com/v1'),
    ('anthropic', 'Anthropic', 'https://api.anthropic.com/v1')
ON CONFLICT (id) DO UPDATE SET
    name = EXCLUDED.name,
    url = EXCLUDED.url;

INSERT INTO models (id, name) VALUES
    ('gpt-4o', 'GPT-4o'),
    ('claude-3-5', 'Claude 3.5 Sonnet')
ON CONFLICT (id) DO UPDATE SET
    name = EXCLUDED.name;

INSERT INTO provider_models (provider_id, model_id) VALUES
    ('openai', 'gpt-4o'),
    ('anthropic', 'claude-3-5'),
    ('openai', 'claude-3-5')
ON CONFLICT (provider_id, model_id) DO NOTHING;
