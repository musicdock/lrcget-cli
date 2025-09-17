# 🎨 Propuesta de Revamp UI Terminal para lrcget-cli

## 📋 **Visión General**

Propongo crear una interfaz de terminal moderna y adaptativa que transforme lrcget-cli de una herramienta de línea de comandos básica a una experiencia visual rica e interactiva, similar a herramientas como `htop`, `bottom` o `lazygit`.

## 🏗️ **Arquitectura de la Interfaz**

### **Layout Adaptativo Principal**
```
┌─────────────────────────────────────────────────────────────────┐
│ 🎵 LRCGET-CLI v1.0.0 │ 📁 /music/library │ 🔗 lrclib.net │ ⏰ 14:35 │ <- Header (3 líneas)
├─────────────────────────────────────────────────────────────────┤
│ 🎯 Current: Downloading "Bohemian Rhapsody" - Queen            │
│ 📊 Progress: ████████████░░░░ 75% │ ⚡ 12.5 songs/min │ 🕐 2m left │
├─────────────────────┬───────────────────┬─────────────────────────┤
│ 📃 PENDING QUEUE    │ 📈 PERFORMANCE    │ 📊 STATISTICS           │
│ (25 lines)          │ (25 lines)        │ (25 lines)              │
│                     │                   │                         │
│ ♪ Song 1 - Artist   │     CPU: █████░   │ ✅ Completed: 450       │
│ ♪ Song 2 - Artist   │     MEM: ███░░░   │ ❌ Failed: 12           │
│ ♪ Song 3 - Artist   │     NET: ██████   │ ⏭️  Skipped: 8          │
│ ♪ Song 4 - Artist   │                   │ 🎵 Synced: 380          │
│ ...                 │   [Real-time      │ 📝 Plain: 70            │
│                     │    line graph]    │ 🎼 Instrumental: 15     │
│ [Scrollable list]   │                   │ ⚡ Avg Speed: 11.2/min  │
├─────────────────────┴───────────────────┴─────────────────────────┤
│ 📋 LOGS & ACTIVITY (8 lines)                                    │
│ [14:35:01] ✅ Downloaded: "Song Name" - Artist                  │
│ [14:35:02] ❌ Failed: "Another Song" - Network timeout          │
│ [14:35:03] ⚠️  Warning: Rate limit approaching                   │
├─────────────────────────────────────────────────────────────────┤
│ ⌨️  CONTROLS: [Space] Pause/Resume │ [ESC] Cancel │ [Q] Quit     │ <- Footer (2 líneas)
│ 📊 [S] Stats │ 🔍 [F] Filter │ 📋 [L] Logs │ ⚙️ [C] Config      │
└─────────────────────────────────────────────────────────────────┘
```

## 📱 **Especificaciones de Paneles**

### 1. **Header Panel (3 líneas)**
- **Línea 1**: Logo/Título + Directorio actual + Servidor + Hora
- **Línea 2**: Operación actual con barra de progreso
- **Línea 3**: Métricas en tiempo real (velocidad, tiempo restante, etc.)

### 2. **Panel de Cola Pendiente (Izquierda)**
- Lista scrolleable de canciones por procesar
- Estado de cada canción (⏳ Pending, 🔄 Processing, ✅ Done, ❌ Failed)
- Colores según estado y prioridad
- Filtros por estado/artista/álbum
- Indicador de posición actual en la lista

### 3. **Panel de Rendimiento (Centro)**
- Gráfico en tiempo real de velocidad de descarga
- Métricas de CPU, memoria y red
- Historial de performance (últimos 60 segundos)
- Detección de throttling y rate limits
- Indicadores de salud del sistema

### 4. **Panel de Estadísticas (Derecha)**
- Contadores en tiempo real (completadas, fallidas, saltadas)
- Breakdown por tipo de lyrics (synced/plain/instrumental)
- Velocidad promedio y actual
- Tiempo total transcurrido
- ETA estimado
- Success rate %

### 5. **Panel de Logs (Parte inferior)**
- Stream en tiempo real de actividad
- Diferentes niveles de log con colores
- Filtros por tipo de mensaje
- Timestamps precisos
- Scroll automático opcional

### 6. **Footer de Controles**
- Hotkeys principales siempre visibles
- Indicadores de estado (paused/running/stopped)
- Contexto sensible (diferentes controles según el modo)

## 🎨 **Sistema de Colores y Temas**

### **Palette Principal**
```rust
// Colores base
Primary: RGB(100, 149, 237)    // Cornflower Blue
Secondary: RGB(60, 179, 113)   // Medium Sea Green
Accent: RGB(255, 165, 0)       // Orange
Danger: RGB(220, 20, 60)       // Crimson
Warning: RGB(255, 215, 0)      // Gold
Success: RGB(50, 205, 50)      // Lime Green
Muted: RGB(169, 169, 169)      // Dark Gray

// Estados
Processing: RGB(135, 206, 250) // Light Sky Blue
Completed: RGB(144, 238, 144)  // Light Green
Failed: RGB(255, 182, 193)     // Light Pink
Pending: RGB(211, 211, 211)   // Light Gray
```

### **Temas Adaptativos**
- **Light Mode**: Para terminales con fondo claro
- **Dark Mode**: Para terminales con fondo oscuro
- **High Contrast**: Para mejor accesibilidad
- **Auto-detection**: Basado en variables de entorno

## ⚙️ **Características Técnicas**

### **Responsive Design**
```rust
// Breakpoints adaptativos
Ultra-Wide (>160 cols): 4 paneles + gráficos extendidos
Wide (120-160 cols):    3 paneles estándar
Medium (80-120 cols):   2 paneles + logs abajo
Narrow (40-80 cols):    1 panel con tabs
Mobile (<40 cols):      Modo simple texto
```

### **Interactividad**
- **Keyboard Navigation**: Navegación completa con teclado
- **Mouse Support**: Scroll, click, resize de paneles
- **Hotkeys Contextuales**: Diferentes según el panel activo
- **Search/Filter**: Filtrado en tiempo real
- **Panel Resizing**: Ajuste manual de tamaños de panel

### **Performance Optimization**
- **Efficient Rendering**: Solo actualizar regiones que cambian
- **Frame Rate Control**: 30-60 FPS configurable
- **Memory Management**: Límites en historial de logs
- **Background Processing**: UI no bloquea operaciones

## 📊 **Tipos de Visualización**

### **Gráficos en Tiempo Real**
1. **Line Chart**: Velocidad de descarga over time
2. **Bar Chart**: Success/failure rates
3. **Sparklines**: CPU/Memory usage mini-graphs
4. **Progress Rings**: Circular progress indicators
5. **Heat Map**: Activity intensity por hora

### **Indicadores Visuales**
- **Progress Bars**: ASCII art con gradientes de color
- **Spinners**: Animaciones suaves para operaciones activas
- **Status Icons**: Emoji/símbolos para estados rápidos
- **Badges**: Contadores con colores de estado
- **Alerts**: Notificaciones no intrusivas

## 🔧 **Implementación Modular**

### **Arquitectura de Componentes**
```rust
src/ui/
├── terminal/           # Nueva UI terminal
│   ├── app.rs         # Aplicación principal
│   ├── layout.rs      # Sistema de layout responsivo
│   ├── panels/        # Paneles individuales
│   │   ├── header.rs
│   │   ├── queue.rs
│   │   ├── performance.rs
│   │   ├── stats.rs
│   │   ├── logs.rs
│   │   └── footer.rs
│   ├── widgets/       # Widgets reutilizables
│   │   ├── progress.rs
│   │   ├── charts.rs
│   │   ├── table.rs
│   │   └── input.rs
│   ├── themes.rs      # Sistema de temas
│   ├── events.rs      # Manejo de eventos
│   └── utils.rs       # Utilidades de rendering
```

### **Estados de la Aplicación**
```rust
enum AppMode {
    Downloading,       // Modo principal de descarga
    Configuration,     // Panel de configuración
    Statistics,        // Vista detallada de estadísticas
    Logs,             // Vista expandida de logs
    Help,             // Ayuda contextual
}

enum PanelFocus {
    Queue,
    Performance,
    Statistics,
    Logs,
    None,
}
```

## 🚀 **Modos de Operación**

### **1. Download Mode (Principal)**
- UI completa como se describe arriba
- Todos los paneles activos
- Actualizaciones en tiempo real

### **2. Configuration Mode**
- Panel central se convierte en editor de configuración
- Preview en tiempo real de cambios
- Validación inline

### **3. Statistics Mode**
- Panel expandido con gráficos detallados
- Exportación de datos
- Comparativas históricas

### **4. Compact Mode**
- UI simplificada para terminales pequeños
- Solo información esencial
- Switching entre vistas

## ⌨️ **Mapeo de Controles**

### **Globales**
- `Space`: Pause/Resume operación
- `ESC ESC`: Cancelar y salir
- `Q`: Quit inmediato
- `H` / `?`: Help contextual
- `Tab`: Cambiar focus entre paneles

### **Panel-específicos**
- `↑/↓`: Scroll en panel activo
- `PgUp/PgDn`: Scroll rápido
- `Home/End`: Ir al inicio/final
- `Enter`: Seleccionar/activar elemento
- `/`: Buscar/filtrar

### **Atajos Rápidos**
- `S`: Toggle statistics panel
- `L`: Toggle logs panel
- `P`: Toggle performance panel
- `C`: Abrir configuración
- `R`: Refresh/reload
- `1-4`: Saltar directamente a panel específico

## 🎯 **Casos de Uso Específicos**

### **Monitoreo de Larga Duración**
- Actualizaciones eficientes sin parpadeo
- Notificaciones de errores importantes
- Auto-scroll inteligente en logs
- Persistencia de estado en crash recovery

### **Debugging y Troubleshooting**
- Vista detallada de errores
- Trace de operaciones individuales
- Export de logs para soporte
- Performance metrics para optimización

### **Operación Desatendida**
- Modo minimizado con progreso básico
- Alertas solo para errores críticos
- Auto-resume después de fallos de red
- Logging completo en background

## 📈 **Métricas y Analytics**

### **Performance Metrics**
- Songs per minute (current/average)
- Success rate percentage
- Network utilization
- Memory usage trends
- Error patterns

### **User Experience Metrics**
- Time to completion
- User interaction patterns
- Most used features
- Error recovery success rate

## 🎁 **Valor Añadido**

### **Para Usuarios Novatos**
- Interfaz visual intuitiva
- Feedback inmediato de progreso
- Guías contextuales y tooltips
- Detección automática de problemas

### **Para Usuarios Avanzados**
- Métricas detalladas de performance
- Controles granulares de operación
- Debugging avanzado
- Personalización completa

### **Para Operaciones Masivas**
- Monitoreo eficiente de largos procesos
- Optimización automática de recursos
- Reporting detallado de resultados
- Recovery automático de errores

Esta propuesta crea una experiencia moderna, profesional e intuitiva que mantiene la funcionalidad de línea de comandos pero añade una capa visual rica para usuarios que prefieren interfaces más interactivas.