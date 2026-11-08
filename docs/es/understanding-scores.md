# Entendiendo las Puntuaciones de Complejidad

## ¿Qué Significan los Números?

Las puntuaciones de complejidad cognitiva representan el esfuerzo mental requerido para entender un fragmento de código. Las puntuaciones más altas indican código más difícil de comprender y mantener.

### Umbrales Recomendados

| Rango de Puntuación | Interpretación | Recomendación |
| -- | -- | -- |
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
| -- | -- | -- |
| Sentencia `if` | +1 | `if condition:` |
| Cláusula `elif` | +1 | `elif other_condition:` |
| Cláusula `else` | +1 | `else:` después de `if`, `for`, `while` o `try` (sin incremento por anidamiento; su cuerpo queda anidado) |
| Bucle `for` | +1 | `for item in items:` |
| Bucle `while` | +1 | `while condition:` |
| Manejador `except` | +1 | `except ValueError:` |
| Cláusula `finally` | +0 | `finally:` (su cuerpo no está anidado) |
| Operador ternario | +1 | `x if condition else y` |
| Cláusula de comprensión | +0 | `[x for x in xs if x]` |

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

- Cada secuencia de operadores iguales, leída de izquierda a derecha: +1
- Cada cambio entre `and` y `or` inicia una nueva secuencia: `a or b and c or d` se lee `or`, `and`, `or` y suma +3

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
# Puntuación: 5
match value:        # +1
    case 1:
        if x:       # +2 (1 + anidamiento)
            pass
    case 2:
        if y:       # +2 (1 + anidamiento)
            pass
```

**Recursión**
Cada función en un ciclo de recursión añade +1 una sola vez, sin importar cuántas llamadas recursivas haga. El ciclo puede ser directo (`fact` llama a `fact`) o indirecto (`a` llama a `b`, `b` llama a `a`). El +1 de una función anidada, o de un método de una clase anidada en una función, va a la función que lo contiene. Una llamada cuenta cuando se resuelve en el mismo archivo: un nombre simple al ámbito de función o de módulo más cercano que lo define, o `self.m()` y `cls.m()` a un método de la clase de quien llama (una clausura dentro de un método usa el `self` de ese método). Una llamada dentro de una lambda, o en los decoradores, los argumentos por defecto, las bases o el cuerpo de clase de una definición anidada, cuenta para la función que la contiene, y los parámetros propios de una lambda ocultan los nombres externos. Un decorador simple como `@f` no es una llamada, así que no cuenta. Las llamadas a través de `super()`, de otros objetos, del nombre de la clase o de imports no cuentan, los ciclos entre archivos no se detectan, y no se rastrea un parámetro o variable que oculte el nombre de una función.

**Decoradores**
Una función cuyo cuerpo es solo un `def` anidado y un `return` no eleva el nivel de anidamiento de la función anidada, como indica la excepción de decoradores de Python del paper. El `return` debe devolver la función anidada: por nombre (`return inner`), o como argumento directo de una llamada (`return functools.wraps(f)(inner)`, `return update_wrapper(inner, f)`). Dos decisiones difieren del texto del paper, que admite cualquier `return` y no nombra nada más. Una llamada a la función anidada, como `return inner()`, no cumple la condición: ejecuta la función anidada en lugar de devolverla, así que la función no es un decorador. Se permite un docstring inicial: documenta el decorador y no añade flujo de control. Cualquier otra sentencia elimina la excepción. Las expresiones del `return` siguen contando.

**Construcciones de Python que el paper no nombra**

- Comprensiones: las cláusulas `for` e `if` no suman nada y no elevan el nivel de anidamiento. El paper ignora las abreviaturas que permiten escribir varias sentencias de forma legible como una sola, y una comprensión es una abreviatura de un bucle que construye un valor. Un ternario o un operador booleano dentro de una comprensión sigue contando.
- `else` en `for`, `while` y `try`: +1, sin incremento por anidamiento, y su cuerpo eleva el nivel de anidamiento, como `else` después de `if`. El cuerpo del `else` solo se ejecuta cuando el bucle termina sin `break` o cuando el cuerpo del `try` no lanzó nada, así que es una rama que el lector debe seguir. El paper llama a `else` un incremento híbrido: sin incremento por anidamiento, pero con un nivel de anidamiento mayor para lo que contiene.
- Guardas de `case`: +0 por la guarda en sí. Una secuencia de operadores booleanos o un ternario dentro de ella cuenta, en el nivel de anidamiento del `match`. El paper da un solo incremento a un `switch` y a todos sus casos y no nombra las guardas, así que una guarda se lee como parte de su caso.
- Operadores booleanos mezclados: los operadores de una condición se leen de izquierda a derecha, en el orden en que aparecen en el código, y cada cambio entre `and` y `or` inicia una nueva secuencia. Un `not` inicia una nueva condición para su operando.

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
    if user.age >= 18 and user.verified and product.available: # +2
        #  ^^^ if: +1, secuencia and: +1, total: +2
        return True
    return False
# Complejidad total: 2
```

**Análisis**: Múltiples operadores booleanos aumentan la carga mental para comprender la condición.

### Ejemplo 4: Manejo de Excepciones

```python
def load_config(path):
    try:                              # try en sí mismo: +0
        with open(path) as f:         # +0 (with no suma nada y no anida)
            data = json.load(f)
            if validate(data):        # +1 (los cuerpos de try y with no están anidados)
                return data
    except FileNotFoundError:         # +1
        create_default_config()
    except json.JSONDecodeError:      # +1
        log_error()
    finally:                          # +0 (finally en sí mismo)
        if log_enabled:               # +1
            log("Config load attempted")
# Complejidad total: 4
```

## Directrices Prácticas

### ¿Qué Debería Refactorizar?

Enfócate en las funciones con puntuaciones por encima de tu umbral (predeterminado: 15). Estrategias comunes de refactorización:

1. **Extraer Métodos** - Mover la lógica anidada a funciones separadas
1. **Retornos Anticipados** - Usar cláusulas de `return` para reducir el anidamiento
1. **Simplificar Condiciones** - Dividir expresiones booleanas complejas en variables con nombre
1. **Patrón Estrategia** - Reemplazar if/else anidados con polimorfismo

### Ejemplo de Refactorización

**Antes (Complejidad: 13)**

```python
def process_order(order):
    if order:                          # +1
        if order.is_valid():           # +2
            if order.payment:          # +3
                if order.payment.process(): # +4
                    ship_order(order)
                else:                          # +1 (cláusula else)
                    refund(order)
            else:                      # +1 (cláusula else)
                notify_payment_required()
        else:                          # +1 (cláusula else)
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
- Complejidad reducida de 13 a 4
- La lógica ahora es lineal y más fácil de seguir

## Consejos para Interpretar Puntuaciones

1. **El Contexto Importa** - Una puntuación de 20 puede ser aceptable para un algoritmo complejo pero problemática para lógica de negocio
1. **Tendencia en el Tiempo** - Observa la complejidad creciente en funciones que modificas frecuentemente
1. **Puntuaciones Relativas** - Compara funciones dentro de la misma base de código para identificar valores atípicos
1. **Acuerdo del Equipo** - Establece umbrales que funcionen para tu equipo y proyecto

## Lecturas Adicionales

- [Documento Técnico de Complejidad Cognitiva de SonarSource](https://www.sonarsource.com/resources/cognitive-complexity/)
- [Complejidad Ciclomática (McCabe, 1976)](https://en.wikipedia.org/wiki/Cyclomatic_complexity)
