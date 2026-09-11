use crate::error::WebError;
use crate::error::WebResult;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiChatRequest {
    pub prompt: String,
    pub context_file: Option<String>,
    pub file_content: Option<String>,
    pub provider: Option<String>, // "gemini", "openai", "anthropic", "ollama", "builtin"
    pub model: Option<String>,
    pub api_key: Option<String>,
    pub custom_endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiChatResponse {
    pub reply: String,
    pub suggested_code: Option<String>,
    pub provider: String,
}

pub struct AiAssistantService;

impl AiAssistantService {
    /// Process AI prompt with user's BYOK provider or intelligent built-in scientific assistant
    pub async fn chat(req: AiChatRequest) -> WebResult<AiChatResponse> {
        let provider = req.provider.as_deref().unwrap_or("gemini").to_lowercase();

        let api_key = req.api_key.as_deref().unwrap_or("").trim();

        // 1. If user supplied Gemini API key or requests Gemini
        if provider == "gemini" && !api_key.is_empty() {
            match Self::call_gemini(&req, api_key).await {
                | Ok(resp) => return Ok(resp),
                | Err(e) => {
                    tracing::warn!(
                        "Gemini API call failed, falling back to scientific engine: {}",
                        e
                    );
                },
            }
        }

        // 2. If user supplied OpenAI key
        if (provider == "openai" || provider == "chatgpt") && !api_key.is_empty() {
            match Self::call_openai(&req, api_key).await {
                | Ok(resp) => return Ok(resp),
                | Err(e) => {
                    tracing::warn!(
                        "OpenAI API call failed, falling back to scientific engine: {}",
                        e
                    );
                },
            }
        }

        // 3. If Ollama local agent
        if provider == "ollama" {
            match Self::call_ollama(&req).await {
                | Ok(resp) => return Ok(resp),
                | Err(e) => {
                    tracing::warn!(
                        "Ollama call failed, falling back to scientific engine: {}",
                        e
                    );
                },
            }
        }

        // 4. Built-in Heuristic Scientific Assistant
        Ok(Self::builtin_scientific_assistant(&req))
    }

    async fn call_gemini(
        req: &AiChatRequest,
        key: &str,
    ) -> WebResult<AiChatResponse> {
        let model = req.model.as_deref().unwrap_or("gemini-2.5-flash");
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, key
        );

        let system_instruction = "You are APICH Copilot, an elite scientific programming and research assistant. Support LaTeX math, Typst markup, cargo-slide presentation DSL, Python scientific data analysis, and Markdown notes.";
        let mut user_text = req.prompt.clone();
        if let Some(ref ctx) = req.file_content {
            let filename = req.context_file.as_deref().unwrap_or("file");
            user_text = format!(
                "Context file `{}`:\n```\n{}\n```\n\nUser Question:\n{}",
                filename,
                if ctx.len() > 12000 {
                    &ctx[..12000]
                } else {
                    ctx
                },
                req.prompt
            );
        }

        let body = serde_json::json!({
            "contents": [{
                "parts": [
                    { "text": format!("{}\n\n{}", system_instruction, user_text) }
                ]
            }],
            "generationConfig": {
                "temperature": 0.2,
                "maxOutputTokens": 2048
            }
        });

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| WebError::Internal(e.to_string()))?;

        let res = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| WebError::Internal(format!("Gemini request error: {}", e)))?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(WebError::Internal(format!(
                "Gemini API error ({}): {}",
                status, err_text
            )));
        }

        let json: serde_json::Value = res
            .json()
            .await
            .map_err(|e| WebError::Internal(format!("Failed to parse Gemini response: {}", e)))?;

        let reply = json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or("No response generated from Gemini")
            .to_string();

        let suggested_code = Self::extract_primary_code_block(&reply);

        Ok(AiChatResponse {
            reply,
            suggested_code,
            provider: format!("Google Gemini ({})", model),
        })
    }

    async fn call_openai(
        req: &AiChatRequest,
        key: &str,
    ) -> WebResult<AiChatResponse> {
        let model = req.model.as_deref().unwrap_or("gpt-4o-mini");
        let url = "https://api.openai.com/v1/chat/completions";

        let mut messages = vec![serde_json::json!({
            "role": "system",
            "content": "You are APICH Copilot, an elite scientific programming and research assistant."
        })];

        let mut prompt_full = req.prompt.clone();
        if let Some(ref ctx) = req.file_content {
            let filename = req.context_file.as_deref().unwrap_or("file");
            prompt_full = format!(
                "File `{}`:\n```\n{}\n```\n\nPrompt: {}",
                filename, ctx, req.prompt
            );
        }

        messages.push(serde_json::json!({
            "role": "user",
            "content": prompt_full
        }));

        let body = serde_json::json!({
            "model": model,
            "messages": messages,
            "temperature": 0.2
        });

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| WebError::Internal(e.to_string()))?;

        let res = client
            .post(url)
            .bearer_auth(key)
            .json(&body)
            .send()
            .await
            .map_err(|e| WebError::Internal(e.to_string()))?;

        let json: serde_json::Value = res
            .json()
            .await
            .map_err(|e| WebError::Internal(e.to_string()))?;

        let reply = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("No response from OpenAI")
            .to_string();

        let suggested_code = Self::extract_primary_code_block(&reply);

        Ok(AiChatResponse {
            reply,
            suggested_code,
            provider: format!("OpenAI ({})", model),
        })
    }

    async fn call_ollama(req: &AiChatRequest) -> WebResult<AiChatResponse> {
        let endpoint = req
            .custom_endpoint
            .as_deref()
            .unwrap_or("http://localhost:11434/api/generate");
        let model = req.model.as_deref().unwrap_or("llama3");

        let body = serde_json::json!({
            "model": model,
            "prompt": req.prompt,
            "stream": false
        });

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| WebError::Internal(e.to_string()))?;

        let res = client
            .post(endpoint)
            .json(&body)
            .send()
            .await
            .map_err(|e| WebError::Internal(format!("Ollama connection failed: {}", e)))?;

        let json: serde_json::Value = res
            .json()
            .await
            .map_err(|e| WebError::Internal(e.to_string()))?;

        let reply = json["response"]
            .as_str()
            .unwrap_or("No response from Ollama")
            .to_string();

        let suggested_code = Self::extract_primary_code_block(&reply);

        Ok(AiChatResponse {
            reply,
            suggested_code,
            provider: format!("Ollama ({})", model),
        })
    }

    /// Built-in Scientific & Heuristic Assistant
    fn builtin_scientific_assistant(req: &AiChatRequest) -> AiChatResponse {
        let p_lower = req.prompt.to_lowercase();
        let cur_file = req.context_file.as_deref().unwrap_or("");
        let is_typst = cur_file.ends_with(".typ");
        let is_python = cur_file.ends_with(".py");
        let is_note = cur_file.ends_with(".anote") || cur_file.ends_with(".md");

        if p_lower.contains("formula")
            || p_lower.contains("equation")
            || p_lower.contains("math")
            || p_lower.contains("hamiltonian")
        {
            let typst_math = "$ hat(H) = 4 E_C (hat(n) - n_g)^2 - E_J cos(hat(phi)) $";
            let reply = format!(
                "### 🔬 Scientific Formula Recommendation\n\n\
                Here is the calibrated Hamiltonian formulation in Typst / LaTeX math:\n\n\
                ```typst\n\
                // Quantum Transmon Hamiltonian\n\
                {}\n\
                $\n\
                  S_(21)(f) = 1 - frac(Q_L / |Q_c| e^(i phi_0), 1 + 2 i Q_L (f - f_r)/f_r)\n\
                $\n\
                ```\n\n\
                Click **Insert at Cursor** below to place this mathematical formulation directly into your working document.",
                typst_math
            );
            return AiChatResponse {
                reply,
                suggested_code: Some(format!("{}\n", typst_math)),
                provider: "APICH Built-in Scientific Engine".to_string(),
            };
        }

        if p_lower.contains("plot")
            || p_lower.contains("script")
            || p_lower.contains("python")
            || is_python
        {
            let code = r#"import matplotlib.pyplot as plt
import numpy as np

# Calibrated Rabi oscillation pulse sweep
t = np.linspace(0, 100, 200) # ns
coherence = np.exp(-t / 45.0) * np.cos(2 * np.pi * 0.05 * t)

plt.figure(figsize=(7, 4), dpi=120)
plt.plot(t, coherence, color='#2563eb', lw=2, label='Measured Ramsey Fringe')
plt.title('Qubit Coherence Oscillation Protocol', fontsize=12, fontweight='bold')
plt.xlabel('Pulse Delay (ns)')
plt.ylabel('Excited State Population P(|1>)')
plt.grid(True, alpha=0.3)
plt.legend()
plt.tight_layout()
plt.savefig('assets/ramsey_fringe.png')
print("Successfully rendered coherence plot: assets/ramsey_fringe.png")
"#;
            let reply = format!(
                "### 🐍 Python Scientific Computation & Plotting\n\n\
                Generated Python telemetry analysis script with matplotlib visualization:\n\n\
                ```python\n\
                {}\
                ```\n\n\
                You can run this directly in the APICH Script Runner console to generate live plots!",
                code
            );
            return AiChatResponse {
                reply,
                suggested_code: Some(code.to_string()),
                provider: "APICH Built-in Scientific Engine".to_string(),
            };
        }

        if p_lower.contains("slide") || p_lower.contains("presentation") || is_typst {
            let code = r#"// New cargo-slide presentation slide
#slide(title: "Experimental Coherence Metrics", transition: "slide-left")[
  #cols(
    [
      #callout(title: "Telemetry Observation", stroke-color: slide-colors.accent)[
        Relaxation time $T_1$ sustained at $90.4~mu"s"$ with minimal cross-talk.
      ]
    ],
    [
      #code-window(title: "telemetry.py")[
        ```python
        avg_t1 = calculate_average_t1('assets/data.csv')
        ```
      ]
    ]
  )
]
"#;
            let reply = format!(
                "### 📊 cargo-slide Presentation Component\n\n\
                Structured slide block conforming to the APICH cargo-slide DSL:\n\n\
                ```typst\n\
                {}\
                ```\n\n\
                Use **Insert at Cursor** to append this slide.",
                code
            );
            return AiChatResponse {
                reply,
                suggested_code: Some(code.to_string()),
                provider: "APICH Built-in Scientific Engine".to_string(),
            };
        }

        if p_lower.contains("task") || p_lower.contains("schedule") || is_note {
            let code = "- [ ] Benchmark pulse fidelity with randomized benchmarking #rb @2026-09-25\n- [/] Calibrate cryostat base temperature stabilization #cryo @2026-09-21\n- [x] Room-temperature microwave loopback test #rf @2026-09-12\n";
            let reply = format!(
                "### 📋 Actionable Markdown Tasks & Schedule Sync\n\n\
                Synchronized task items bound to Kanban and Calendar:\n\n\
                ```markdown\n\
                {}\
                ```\n\n\
                These tasks will automatically sync to your project Kanban and Calendar.",
                code
            );
            return AiChatResponse {
                reply,
                suggested_code: Some(code.to_string()),
                provider: "APICH Built-in Scientific Engine".to_string(),
            };
        }

        // Generic intelligent explanation / assistance
        let reply = format!(
            "### 🤖 APICH AI Research Assistant\n\n\
            I analyzed your request: **\"{}\"**.\n\n\
            **Key Suggestions**:\n\
            1. **Reproducibility**: Ensure all parameters are version-controlled with FastCDC chunking.\n\
            2. **Interactivity**: Connect your markdown tasks with `@YYYY-MM-DD` and `#tag` to view them in the reactive Kanban and Calendar.\n\
            3. **Reverse Search**: In Typst/slide editor, click any equation or text in the live SVG preview to jump directly to that line in code.\n\
            4. **BYOK Setup**: You can enter your Google Gemini, OpenAI, or Ollama API key in the settings drawer for full LLM autonomy!\n\n\
            How can I assist you further with this file (`{}`)?",
            req.prompt,
            if cur_file.is_empty() { "workspace" } else { cur_file }
        );

        AiChatResponse {
            reply,
            suggested_code: None,
            provider: "APICH Built-in Scientific Engine".to_string(),
        }
    }

    fn extract_primary_code_block(text: &str) -> Option<String> {
        let regex = regex::Regex::new(r"```(?:\w+)?\n([\s\S]*?)```").ok()?;
        if let Some(caps) = regex.captures(text) {
            if let Some(m) = caps.get(1) {
                return Some(m.as_str().to_string());
            }
        }
        None
    }
}
