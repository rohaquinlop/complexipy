# Entendiendo las Puntuaciones de Complejidad

## ¿Qué Significan los Números?

Las puntuaciones de complejidad cognitiva representan el esfuerzo mental requerido para entender un fragmento de código. Las puntuaciones más altas indican código más difícil de comprender y mantener.

### Umbrales Recomendados

| Rango de Puntuación | Interpretación | Recomendación |
|---------------------|----------------|---------------|
| **0-5** | Simple | Fácil de entender, no se requiere acción |
| **6-10** | Moderado | Generalmente aceptable, pero con vigilancia ante mayor crecimiento |
| **11-15** | Complejo | Considerar refactorización si se está añadiendo funcionalidad |
| **16-25** | Alto | Refactorización recomendada |
| **26+** | Muy Alto | Refactorización fuertemente recomendada |

!!! note "Umbral Predeterminado"
    complexipy usa un umbral predeterminado de **15**. Las funciones que superen esta puntuación activarán advertencias.

## Cómo se Calcula la Complejidad

La complejidad cognitiva se calcula analizando el Árbol de Sintaxis Abstracta (AST) de tu código Python y aplicando estas reglas:

### 1. Incrementos de Complejidad Base

Cada estructura de flujo de control añade a la complejidad:

| Estructura | Puntuación | Ejemplo |
|------------|------------|---------|
| Sentencia `if` | +1 | `if condition:` |
| Cláusula `elif` | +1 | `elif other_condition:` |
| Cláusula `else` | +0 | `else:` (solo anidamiento) |
| Bucle `for` | +1 | `for item in items:` |
| Bucle `while` | +1 | `while condition:` |
| Manejador `except` | +1 | `except ValueError:` |
| Cláusula `finally` | +0 | `finally:` (solo anidamiento) |
| Operador ternario | +1 | `x if condition else y` |

### 2. Multiplicador de Anidamiento

**Esta es la innovación clave**: las estructuras anidadas añaden complejidad extra basada en su profundidad.

```python
def example():
    if a:           # +1 (base)
        if b:       # +2 (1 base + 1 anidamiento)
            if c:   # +3 (1 base + 2 anidamiento)
                pass
# Complejidad total: 6
```

La fórmula es: `complexity = 1 + nesting_level`

### 3. Operadores Booleanos

Las condiciones lógicas complejas aumentan la carga cognitiva:

```python
# Puntuación: 3
def check(a, b, c):
    if a and b or c:  # +1 (if) +2 (operadores booleanos)
        return True
```

- Cada operador `and` u `or`: +1
- Cambios de tipo de operador (mezclar `and`/`or`): +1 adicional

### 4. Casos Especiales

**Break y Continue**
```python
# Puntuación: 3
for item in items:  # +1
    if condition:   # +2 (1 + anidamiento)
        break       # +0 (sin puntuación adicional)
```

**Sentencias Match** (Python 3.10+)
```python
# Puntuación: 2
match value:        # +0 (el match en sí no cuenta)
    case 1:
        if x:       # +1
            pass
    case 2:
        if y:       # +1
            pass
```

**Recursión**
Las llamadas recursivas se analizan pero no añaden complejidad extra más allá de sus estructuras de control.

## Ejemplos Detallados

### Ejemplo 1: Lógica Secuencial Simple

```python
def process_user(user):
    if not user:              # +1
        return None

    if user.is_active:        # +1
        send_welcome_email()

    if user.needs_verification: # +1
        send_verification()

    return user
# Complejidad total: 3
```

**Análisis**: Tres sentencias if secuenciales, sin anidamiento. Fácil de seguir.

### Ejemplo 2: Lógica Anidada

```python
def validate_order(order):
    if order:                        # +1
        if order.items:              # +2 (1 + anidamiento)
            for item in order.items: # +3 (1 + anidamiento)
                if item.quantity > 0: # +4 (1 + anidamiento)
                    process(item)
    return False
# Complejidad total: 10
```

**Análisis**: El anidamiento profundo crea un crecimiento exponencial en la carga cognitiva.

### Ejemplo 3: Condiciones Complejas

```python
def check_eligibility(user, product):
    if user.age >= 18 and user.verified and product.available: # +4
        #  ^^^ if: +1, operadores and: +2, total: +4
        return True
    return False
# Complejidad total: 4
```

**Análisis**: Múltiples operadores booleanos aumentan la carga mental para comprender la condición.

### Ejemplo 4: Manejo de Excepciones

```python
def load_config(path):
    try:                              # try en sí mismo: +0
        with open(path) as f:         # +1 (gestor de contexto tratado como if)
            data = json.load(f)
            if validate(data):        # +2 (1 + anidamiento)
                return data
    except FileNotFoundError:         # +1
        create_default_config()
    except json.JSONDecodeError:      # +1
        log_error()
    finally:                          # +0 (finally en sí mismo)
        if log_enabled:               # +1
            log("Config load attempted")
# Complejidad total: 6
```

## Directrices Prácticas

### ¿Qué Debería Refactorizar?

Enfócate en las funciones con puntuaciones por encima de tu umbral (predeterminado: 15). Estrategias comunes de refactorización:

1. **Extraer Métodos** - Mover la lógica anidada a funciones separadas
2. **Retornos Anticipados** - Usar cláusulas de `return` para reducir el anidamiento
3. **Simplificar Condiciones** - Dividir expresiones booleanas complejas en variables con nombre
4. **Patrón Estrategia** - Reemplazar if/else anidados con polimorfismo

### Ejemplo de Refactorización

**Antes (Complejidad: 12)**
```python
def process_order(order):
    if order:                          # +1
        if order.is_valid():           # +2
            if order.payment:          # +3
                if order.payment.process(): # +4
                    ship_order(order)
                else:
                    refund(order)      # +0 (cláusula else)
            else:
                notify_payment_required() # Aún anidado
        else:
            log_invalid_order()
    return False
```

**Después (Complejidad: 4)**
```python
def process_order(order):
    if not order:                  # +1
        return False

    if not order.is_valid():       # +1
        log_invalid_order()
        return False

    if not order.payment:          # +1
        notify_payment_required()
        return False

    if order.payment.process():    # +1
        ship_order(order)
        return True

    refund(order)
    return False
```

**Mejoras clave:**
- Anidamiento reducido de 4 niveles a 1
- Complejidad reducida de 12 a 4
- La lógica ahora es lineal y más fácil de seguir

## Consejos para Interpretar Puntuaciones

1. **El Contexto Importa** - Una puntuación de 20 puede ser aceptable para un algoritmo complejo pero problemática para lógica de negocio
2. **Tendencia en el Tiempo** - Observa la complejidad creciente en funciones que modificas frecuentemente
3. **Puntuaciones Relativas** - Compara funciones dentro de la misma base de código para identificar valores atípicos
4. **Acuerdo del Equipo** - Establece umbrales que funcionen para tu equipo y proyecto

## Lecturas Adicionales

- [Documento Técnico de Complejidad Cognitiva de SonarSource](https://www.sonarsource.com/resources/cognitive-complexity/)
- [Complejidad Ciclomática (McCabe, 1976)](https://en.wikipedia.org/wiki/Cyclomatic_complexity)
