# Proyecto3 - Sistema (Simulación del Sistema Solar)

Pequeña simulación/visualización en Rust que renderiza un sistema planetario en 3D.

## Qué hay aquí
- `src/` - código fuente en Rust (render, cámaras, shaders, objetos, UI mínima).
- `assets/` - modelos y recursos (OBJ/MTL). Aquí van las texturas y modelos (ej: `SpaceShip.obj`, `sphere.obj`, `Asteroid.obj`).
- `Cargo.toml` - dependencias y configuración de compilación.

## Requisitos
- Rust (stable) y Cargo instalados. (https://www.rust-lang.org)
- Dependencias gestionadas por Cargo.
- Recomendado: compositor (Wayland/X11) para Linux.

## Cómo compilar y ejecutar
Desde la carpeta del proyecto:

```bash
# Compilar en modo release para mejor rendimiento
cargo build --release
# Ejecutar
cargo run --release
```


## Controles
- Movimiento de la nave:
  - W / S: empuje adelante/atrás (W empuja hacia la nariz de la nave)
  - A / D: strafear izquierda/derecha
  - R / F: subir/bajar
  - Flechas Izq/Dcha: girar (yaw)
  - Flechas Arriba/Abajo: pitch (Up incrementa pitch, es decir "subir la nariz")
  - Left Shift: boost (multiplica aceleración)
- Cámara y navegación:
  - C: volver a la cámara que sigue la nave
  - Teclas 0..8: seleccionar y "warp" para seguir cada planeta (0 = Sol, 1 = Mercurio, ... 8 = Neptuno)
  - O: alterna animación de órbitas
  - Escape: salir
- Utilidades:
  - S: guardar screenshot actual como `screenshot.png`

## Asteroides
- Se generan asteroides que cruzan frente a la nave.
- Generados del mismo tamaño que Venus (ajustable en `src/main.rs`).
- Estan limitados a 2 asteroides al mismo tiempo.
- Al acercarse, explotan con un efecto de glow y desaparecen.

> Para cambiar el tamaño de los asteroides o su comportamiento, edita `src/main.rs` (funciones de spawn y constantes).

## Screenshots


## Video







