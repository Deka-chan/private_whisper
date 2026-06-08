privatewhisper (lite) — локальная диктовка на GPU, лёгкий дистрибутив
=====================================================================

ОТЛИЧИЕ ОТ ПОЛНОГО БАНДЛА
  Здесь только exe (~26 МБ). Тяжёлое приложение скачивает само ПРИ ПЕРВОМ
  ЗАПУСКЕ в %LOCALAPPDATA%\privatewhisper\:
    • модель Parakeet TDT v3 (fp16) — ~1.24 ГБ  -> ...\models\parakeet-v3
    • GPU-рантайм (onnxruntime CUDA-13 + CUDA13/cuDNN9) — ~1.2 ГБ -> ...\runtime
  Итого первый запуск тянет ~2.5 ГБ. Дальше запуски быстрые, без загрузок.

ТРЕБОВАНИЯ
  • Windows 10/11 x64
  • NVIDIA RTX 50-й серии (Blackwell) + свежий драйвер (CUDA 13-capable)
  • Microsoft Visual C++ Redistributable 2015–2022 (x64):
        https://aka.ms/vs/17/release/vc_redist.x64.exe
  • ~2.5 ГБ трафика на первый запуск

ЗАПУСК
  1. Скопируй папку на диск Windows (например C:\privatewhisper).
  2. Запусти run.bat (покажет прогресс загрузки в консоли).
  3. Дождись, пока докачаются рантайм и модель (видно в логах).
  4. Появится иконка в трее. Ctrl+Space — старт записи (внизу побежит
     waveform-индикатор), Ctrl+Space ещё раз — распознавание и вставка по курсору.

ЕСЛИ НУЖНО БЕЗ ЗАГРУЗКИ
  Используй полный бандл (папка privatewhisper рядом), где GPU-рантайм уже лежит
  рядом с exe и его run.bat указывает на него — тогда качается только модель.

НАСТРОЙКИ: %APPDATA%\privatewhisper\config.toml (hotkey, execution_provider, ...).
Логи: запусти из консоли, смотри строки "runtime wheel i/N", "model file ...",
"ASR ready on Cuda".
