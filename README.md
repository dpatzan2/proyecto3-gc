# Proyecto3 - Sistema (Simulación del Sistema Solar)

Pequeña simulación/visualización en Rust que renderiza un sistema planetario en 3D.



https://github.com/user-attachments/assets/3ce6defc-dcd3-4fae-a2bc-3c3407d4a656



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
<img width="900" height="700" alt="screenshot" src="https://github.com/user-attachments/assets/41268555-b657-4303-8368-7c24608d6cd4" />

<img width="900" height="700" alt="screenshot" src="https://github.com/user-attachments/assets/425fa189-ec7a-4999-ac5f-5773dfe74cc0" />

<img width="900" height="700" alt="screenshot" src="https://github.com/user-attachments/assets/773cb466-1d0c-40b4-8512-0e82c19bb03a" />

<img width="900" height="700" alt="screenshot" src="https://github.com/user-attachments/assets/f41fb76d-dfb8-4da5-8809-4f705e40acdd" />

<img width="900" height="700" alt="screenshot" src="https://github.com/user-attachments/assets/1a0d4ef8-9c44-44e2-befc-aa2798594ecd" />

<img width="900" height="700" alt="screenshot" src="https://github.com/user-attachments/assets/e3d093f0-7c01-4428-b2dd-70f39104c268" />

<img width="900" height="700" alt="screenshot" src="https://github.com/user-attachments/assets/e9c1b6a4-25eb-498a-adc7-fc679b64642f" />

<img width="900" height="700" alt="screenshot" src="https://github.com/user-attachments/assets/63c1ba36-531f-48b1-8c34-5f2e7ed0c86c" />

<img width="900" height="700" alt="screenshot" src="https://github.com/user-attachments/assets/7fd57f82-6367-4ed0-8003-70614b80ec69" />








