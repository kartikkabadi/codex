# 0001 Native Protocol Scope

Native Protocol support is limited to provider-specific non-OpenAI APIs, currently `anthropic-messages` and `gemini`. OpenAI-Compatible Providers using `openai-chat-completions` remain outside this scope until the project defines and tests an explicit adapter contract for that protocol. This keeps the routing semantics honest: compatibility with OpenAI-shaped APIs is not the same thing as native provider support, and routing those providers through the native path prematurely would make failures harder to diagnose.

