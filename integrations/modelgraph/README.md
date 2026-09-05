# ModelGraph — интеграция с LLM

Пакет подключает локальный MCP к Claude Desktop, Claude Code, Cursor, VS Code и Codex. Он содержит skill с инструкцией для модели и манифесты Codex/Claude Code. Конфигурации соответствуют форматам клиентов; фактическая установка в этих приложениях отдельно не проверялась.

## Локальный пакет

Из корня установленного проекта:

```sh
npm ci
node scripts/package-modelgraph-plugin.mjs
node scripts/smoke-modelgraph-plugin.mjs
```

Результаты находятся в `.local-integrations/`:

- `modelgraph/` — плагин с `.codex-plugin/plugin.json`, `.claude-plugin/plugin.json`, MCP-конфигурацией и skill.
- `claude-desktop.json` — добавить запись `mcpServers.modelgraph` в конфигурацию Claude Desktop.
- `claude-code.json` — запись для проектного `.mcp.json` Claude Code.
- `cursor.json` — запись для `.cursor/mcp.json`.
- `vscode.json` — запись для `.vscode/mcp.json`, в секцию `servers`.
- `codex.toml` — секция MCP для конфигурации Codex.

Объединяйте записи с существующими настройками, не заменяйте весь файл. Не включайте одновременно плагин и отдельную конфигурацию того же сервера. Пакет не устанавливает себя и не изменяет настройки клиентов.

Конфигурации содержат абсолютные пути к Node.js, плагину и проекту. Проект с установленными зависимостями должен оставаться на месте. После переноса запускайте упаковку с новым каталогом назначения: `node scripts/package-modelgraph-plugin.mjs /absolute/new/directory`. Это пакет для установленного checkout, не автономный MCPB с включёнными нативными зависимостями. Архивирование папки не делает её переносимой на другой компьютер.

Каждый локальный клиент получает отдельный временный каталог моделей. Документы и нужные экспортированные файлы следует сохранять в проект; каталог MCP исчезает при остановке процесса.

## HTTP для веб-чатов

```sh
node --import tsx src/mcp/modelGraphHttp.ts
```

Адрес на этой машине: `http://127.0.0.1:7433/mcp`. Порт меняется через `MODELGRAPH_PORT`. Процесс слушает только loopback. Для веб-клиентов разместите перед ним HTTPS reverse proxy либо согласованный туннель. Веб-клиенту указывается HTTPS URL с путём `/mcp`, а не локальная JSON-конфигурация.

HTTP-профиль предоставляет `modelgraph_language`, `modelgraph_compile`, `modelgraph_check`, `modelgraph_set_parameters`, `modelgraph_report`, `modelgraph_export`. Описание языка доступно как инструмент и ресурс, чтобы не зависеть от поддержки ресурсов клиентом. Экспорт возвращает встроенный MCP-ресурс STL/3MF/OBJ/PLY/OFF/AMF, до 4 MiB; отображение/скачивание вложения зависит от клиента. Постоянного хранилища и публичных ссылок на файлы нет.

HTTP-профиль stateless, без собственной авторизации. Он рассчитан на доверенное подключение через туннель или шлюз доступа. Для публичного многопользовательского размещения требуется настроить аутентификацию на шлюзе, совместимую с целевым клиентом (например OAuth). Не считайте наличие HTTPS авторизацией. Ограничения: 2 одновременных HTTP-запроса, тело до 256 KiB, таймаут 30 секунд; геометрия исполняется в изолированном worker. Запросы с браузерным Origin отклоняются — это серверное MCP-подключение, не прямой fetch из страницы.

Публичный адрес, туннель и OAuth этим пакетом автоматически не создаются. Подключение к аккаунтам ChatGPT/Claude и их ограничениям нужно проверять после выбора размещения.

## Проверки

`node scripts/smoke-modelgraph-plugin.mjs` запускает именно сгенерированную конфигурацию из постороннего рабочего каталога, читает ресурс языка, строит реальную геометрию и проверяет структурированную ошибку ограничения.

`npx vitest run tests/modelGraphHttp.test.ts` проверяет HTTP handshake, обнаружение инструментов, построение, STL-экспорт и ограничения входящих запросов.

## Документация клиентов

- [Claude local MCP](https://support.claude.com/en/articles/10949351-getting-started-with-local-mcp-servers-on-claude-desktop)
- [Claude Code MCP](https://code.claude.com/docs/en/mcp)
- [Cursor MCP](https://prod.cursor.com/docs/mcp)
- [VS Code MCP](https://code.visualstudio.com/docs/agent-customization/mcp-servers)
- [ChatGPT: подключение и проверка](https://developers.openai.com/plugins/deploy/connect-chatgpt)

Наличие инструмента в клиенте не гарантирует, что любая выбранная LLM корректно использует схему. Печатность не гарантируется проверкой выражений; modelgraph_report возвращает ограниченные PNG-превью, если клиент поддерживает изображения.

## Собственное NURBS-ядро

`modelgraph_nurbs_language` передаёт схему и примеры отдельного контракта `modelgraph/nurbs-1`. Расчёт рациональных кривых/поверхностей, редактирование, тесселяция и STL реализованы в проекте на TypeScript. Дополнительный геометрический пакет или Python не нужны. `modelgraph_nurbs_build` возвращает определения, диагностику сетки и PNG; `modelgraph_nurbs_export` — исходный JSON или поддерживаемый формат сетки. STEP и общие B-rep boolean-операции пока не реализованы. Проверка: `npx vitest run tests/nurbsCurve.test.ts tests/nurbsSurface.test.ts tests/nurbsTessellation.test.ts tests/modelGraphNurbs.test.ts tests/modelGraphNurbsMcp.test.ts`.

## Форматы экспорта

`modelgraph_export` и `modelgraph_nurbs_export`: `stl` (ASCII), `stl_binary`, `3mf`, `obj`, `ply`, `off`, `amf`. NURBS также сохраняет исходный `json`. STL/3MF/AMF требуют замкнутую ориентированную сетку. Файл содержит геометрию, без настроек слайсера, текстур и семантики сборки. Максимум 4 МиБ и 100000 треугольников. В панели браузера доступны STL/OBJ и выбор 3MF/PLY/OFF/AMF.
