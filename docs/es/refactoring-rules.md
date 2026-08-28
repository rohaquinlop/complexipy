# Reglas de Refactorización

complexipy incluye un sistema de refactorización inspirado en clippy que proporciona sugerencias accionables para reducir la complejidad cognitiva. Cada regla tiene un ID único, categoría y nivel de aplicabilidad.

## Categorías de Reglas

| Categoría | Icono | Descripción |
| -- | -- | -- |
| **Complejidad** | ▲ | Reglas que reducen directamente la complejidad cognitiva |
| **Legibilidad** | ◆ | Reglas que mejoran la legibilidad del código |

## Niveles de Aplicabilidad

| Nivel | Icono | Descripción |
| -- | -- | -- |
| **Seguro de aplicar** | \* | Alta confianza en que el código generado es correcto tal cual -- todavía no se aplica automáticamente, es una señal de confianza, no una promesa de automatización |
| **Necesita revisión** | ! | Puede ser incorrecto en algunos casos, necesita revisión humana |
| **Informativo** | i | Solo orientación, no es directamente accionable |

______________________________________________________________________

## Reglas de Complejidad

### C001: Aplanar Condiciones Anidadas

- **Categoría:** ▲ Complejidad
- **Aplicabilidad:** i Informativo
- **Prioridad:** Alta (4/5)

Aplana bloques de condiciones anidadas usando cláusulas de guarda con retornos anticipados.

#### ¿Cuándo se activa?

Esta regla se activa cuando una función tiene sentencias `if` profundamente anidadas (2+ niveles de anidamiento) que añaden complejidad significativa.

#### Ejemplo

**Antes:**

```python
def process_data(data):
    if data:
        if data.is_valid():
            if data.is_ready():
                return process(data)
    return None
```

**Después:**

```python
def process_data(data):
    if not data:
        return None
    if not data.is_valid():
        return None
    if not data.is_ready():
        return None
    return process(data)
```

#### Por qué esto ayuda

Las condiciones profundamente anidadas son difíciles de seguir. Usar cláusulas de guarda con retornos anticipados reduce la carga cognitiva al mantener la ruta principal en un nivel de indentación menor.

______________________________________________________________________

### C002: Guardas de Bucles

- **Categoría:** ▲ Complejidad
- **Aplicabilidad:** * Seguro de aplicar
- **Prioridad:** Media (3/5)

Usa guardas `continue` al inicio de los bucles para reducir el anidamiento.

#### ¿Cuándo se activa?

Esta regla se activa cuando un bucle contiene sentencias `if` anidadas que podrían convertirse en guardas `continue` anticipadas.

#### Ejemplo

**Antes:**

```python
def process_items(items):
    total = 0
    for item in items:
        if item.active:
            if item.ready:
                total += item.value
    return total
```

**Después:**

```python
def process_items(items):
    total = 0
    for item in items:
        if not (item.active):
            continue
        if not (item.ready):
            continue
        total += item.value
    return total
```

#### Por qué esto ayuda

Las condiciones anidadas dentro de los bucles añaden indentación innecesaria. Usar guardas `continue` mantiene la lógica principal en un nivel de anidamiento menor y hace que el bucle sea más fácil de seguir.

______________________________________________________________________

### C003: Extraer Función Auxiliar

- **Categoría:** ▲ Complejidad
- **Aplicabilidad:** i Informativo
- **Prioridad:** Baja (2/5)

Extrae bloques de código complejos en funciones auxiliares separadas.

#### ¿Cuándo se activa?

Esta regla se activa cuando un bloque de código tiene alta complejidad (6+) y abarca múltiples líneas (5+).

#### Ejemplo

**Antes:**

```python
def process_order(order):
    # Complex validation and processing logic
    if order.items:
        for item in order.items:
            if item.quantity > 0:
                if item.price > 0:
                    total = item.quantity * item.price
                    if total > 100:
                        apply_discount(total)
                    process_item(item)
```

**Después:**

```python
def process_order(order):
    if not order.items:
        return
    for item in order.items:
        process_order_item(item)

def process_order_item(item):
    if item.quantity <= 0 or item.price <= 0:
        return
    total = item.quantity * item.price
    if total > 100:
        apply_discount(total)
    process_item(item)
```

#### Por qué esto ayuda

Los bloques de código complejos deben extraerse en funciones con nombre para mejorar la legibilidad y la capacidad de prueba. La función extraída puede recibir un nombre descriptivo que explique su propósito.

______________________________________________________________________

### C004: Dividir Despachador

- **Categoría:** ▲ Complejidad
- **Aplicabilidad:** i Informativo
- **Prioridad:** Baja (2/5)

Divide cadenas largas de `elif` en manejadores separados.

#### ¿Cuándo se activa?

Esta regla se activa cuando una cadena `if/elif` tiene 3+ ramas. Las sentencias `match`
se excluyen intencionalmente: el modelo de complejidad cognitiva de complexipy asigna a
un `match` un costo fijo sin importar cuántas cláusulas `case` tenga (a diferencia de
una cadena `elif`, donde cada rama adicional suma a la puntuación), así que dividir un
`match` en manejadores separados no reduciría realmente la complejidad medida.

El refactor sugerido depende de la forma de la cadena:

- Si todas las ramas comparan la **misma variable** contra un **literal** con `==`, la
  cadena puede convertirse en una sentencia `match` -- esto se recomienda por encima de
  un diccionario de despacho, ya que no necesita indirección adicional y el propio
  modelo de complexipy la puntúa sin el costo por rama que tiene la cadena `elif`.
- En caso contrario (rangos, varias variables, comparaciones que no son de igualdad --
  cualquier cosa que un simple `match case <valor>:` no pueda expresar), se sugiere en
  su lugar un diccionario de despacho que asigna casos a funciones manejadoras.

#### Ejemplo: igualdad de una sola variable → `match`

**Antes:**

```python
def handle_action(action):
    if action == "create":
        return create_resource()
    elif action == "read":
        return read_resource()
    elif action == "update":
        return update_resource()
    elif action == "delete":
        return delete_resource()
    return None
```

**Después:**

```python
def handle_action(action):
    match action:
        case "create":
            return create_resource()
        case "read":
            return read_resource()
        case "update":
            return update_resource()
        case "delete":
            return delete_resource()
    return None
```

#### Ejemplo: rangos → diccionario de despacho

**Antes:**

```python
def classify(score):
    if score < 60:
        return "fail"
    elif score < 70:
        return "d"
    elif score < 85:
        return "b"
    elif score < 95:
        return "a"
    return "a+"
```

**Después:**

```python
def classify(score):
    thresholds = [(60, "fail"), (70, "d"), (85, "b"), (95, "a")]
    for limit, grade in thresholds:
        if score < limit:
            return grade
    return "a+"
```

#### Por qué esto ayuda

Las cadenas condicionales largas son difíciles de mantener y extender. Dividirlas en manejadores separados -- una sentencia `match` cuando la cadena es un despacho simple por igualdad, un diccionario de despacho en caso contrario -- hace que cada caso sea independientemente testeable y la lógica de despacho más clara.

______________________________________________________________________

### C011: Aplanar Try/Except

- **Categoría:** ▲ Complejidad
- **Aplicabilidad:** i Informativo
- **Prioridad:** Baja (2/5)

Aplana bloques try/except anidados combinándolos o reestructurándolos.

#### ¿Cuándo se activa?

Esta regla se activa cuando los bloques try/except están anidados unos dentro de otros.

#### Ejemplo

**Antes:**

```python
def read_config(path):
    try:
        with open(path) as f:
            try:
                return json.load(f)
            except json.JSONDecodeError:
                return default_config()
    except FileNotFoundError:
        return default_config()
```

**Después:**

```python
def read_config(path):
    try:
        with open(path) as f:
            return json.load(f)
    except (FileNotFoundError, json.JSONDecodeError):
        return default_config()
```

#### Por qué esto ayuda

Los bloques try/except anidados son confusos y difíciles de mantener. Considera fusionarlos o extraer el bloque interno en una función separada con su propio manejo de errores.

______________________________________________________________________

## Reglas de Legibilidad

### C005: Extraer Predicado

- **Categoría:** ◆ Legibilidad
- **Aplicabilidad:** * Seguro de aplicar
- **Prioridad:** Baja (2/5)

Extrae condiciones booleanas complejas en funciones predicado con nombre.

#### ¿Cuándo se activa?

Esta regla se activa cuando una condición booleana contiene 2+ operadores lógicos (and, or, not).

La sugerencia conserva la palabra clave de la sentencia: una condición `if` sigue siendo `if`, una condición `while` sigue siendo `while`. Las condiciones en líneas `elif` solo reciben texto de ayuda, porque una función extraída no puede colocarse dentro de una cadena if.

#### Ejemplo

**Antes:**

```python
def is_eligible(user, order):
    if (user.is_active and user.has_subscription) or (order.total > 100 and not order.is_gift):
        return True
    return False
```

**Después:**

```python
def is_eligible(user, order):
    return has_active_subscription(user) or is_qualifying_order(order)

def has_active_subscription(user):
    return user.is_active and user.has_subscription

def is_qualifying_order(order):
    return order.total > 100 and not order.is_gift
```

#### Por qué esto ayuda

Las expresiones booleanas complejas son difíciles de entender a simple vista. Extraerlas en predicados con nombre hace que el código sea autoexplicativo y más fácil de probar.

______________________________________________________________________

### C007: Aplanar If Anidados

- **Categoría:** ◆ Legibilidad
- **Aplicabilidad:** * Seguro de aplicar
- **Prioridad:** Máxima (5/5)

Combina sentencias `if` anidadas en un único `if` con las condiciones combinadas.

#### ¿Cuándo se activa?

Esta regla se activa cuando el cuerpo completo de una sentencia `if` es un único `if` anidado sin rama `else`, para una cadena de dos o más niveles de este tipo.

La fusión se omite cuando hay alguna sentencia o comentario entre el `if` exterior y el interior, de modo que la sugerencia nunca elimina código.

#### Ejemplo

**Antes:**

```python
def check_eligibility(user):
    if user.is_active:
        if user.has_permission:
            return True
    return False
```

**Después:**

```python
def check_eligibility(user):
    if user.is_active and user.has_permission:
        return True
    return False
```

#### Por qué esto ayuda

Las sentencias `if` anidadas con un único cuerpo pueden combinarse en un único `if` con las condiciones unidas mediante `and`. Esto reduce el anidamiento y mejora la legibilidad.

______________________________________________________________________

## Uso de las Reglas de Refactorización

### Línea de Comandos

```bash
# Muestra sugerencias de refactorización
complexipy . --suggest-refactors

# Muestra sugerencias solo para funciones fallidas
complexipy . --failed --suggest-refactors

# Exporta sugerencias a JSON
complexipy . --output-format json --suggest-refactors
```

### API de Python

```python
from complexipy import code_complexity

code = """
def process(data):
    if data:
        if data.is_valid():
            return process(data)
    return None
"""

result = code_complexity(code)
for func in result.functions:
    for plan in func.refactor_plans:
        print(f"[{plan.rule_id}] {plan.title}")
        print(f"  Category: {plan.category}")
        print(f"  Applicability: {plan.applicability}")
        print(f"  Reduction: -{plan.estimated_reduction} complexity")
        if plan.suggestion:
            print(f"  Suggested replacement:\n{plan.suggestion.replacement}")
        elif plan.help:
            print(f"  Help: {plan.help}")
        print(f"  Docs: {plan.doc_url}")
```

### Salida JSON

La salida JSON incluye todos los metadatos de las reglas para consumo programático:

```json
{
  "rule_id": "C007",
  "kind": "collapsible_if",
  "title": "Merge nested if statements",
  "category": "Readability",
  "applicability": "MachineApplicable",
  "description": "Merge nested if statements into a single if with combined conditions",
  "line_start": 3,
  "line_end": 5,
  "column_start": 5,
  "current_complexity": 4,
  "estimated_reduction": 1,
  "estimated_complexity_after": 3,
  "reduction_is_measured": true,
  "suggestion": {
    "replacement": "    if data and data.is_valid():\n        return process(data)",
    "applicability": "MachineApplicable",
    "spliceable": true,
    "description": "Merge nested conditions into `if data and data.is_valid():`"
  },
  "help": null,
  "explanation": "Nested if statements with a single body can be merged into a single if with combined conditions using 'and'. This reduces nesting and improves readability.",
  "references": [],
  "doc_url": "https://rohaquinlop.github.io/complexipy/refactoring-rules/#c007-collapsible-if"
}
```

### Reducciones medidas vs estimadas

Cada plan informa cuánto reduce la complejidad al aplicarlo
(`estimated_reduction` / `estimated_complexity_after`) junto con un
indicador de confianza, `reduction_is_measured`:

- **Medida** (`true`): la regla insertó su sugerencia en el código real, lo
  re-analizó y volvió a ejecutar el puntuador. El número es la respuesta
  literal a "aplica esto y vuelve a puntuar" — exactamente lo que
  reportan C002 y C007, que llevan reemplazos aplicables por máquina. La
  CLI los muestra sin calificador: `Reduction: -2 complexity (7 -> 5)`.
- **Estimada** (`false`): el número proviene de la fórmula manual de la
  regla (reglas solo con ayuda C001, C003, C004, C011, y el fragmento
  C005, además de cualquier respaldo cuando una inserción no se puede
  re-analizar). La CLI los muestra con una tilde:
  `Estimated reduction: ~-2 complexity (7 -> 5)`.

Una reducción medida de 0 significa que la sugerencia no reduce la
complejidad realmente; el plan se descarta en lugar de mostrarse.

______________________________________________________________________

## Referencia de IDs de Reglas

| ID | Nombre | Categoría | Aplicabilidad | Prioridad |
| -- | -- | -- | -- | -- |
| [C001](#c001-aplanar-condiciones-anidadas) | Aplanar Condiciones Anidadas | ▲ Complejidad | i Informativo | Alta |
| [C002](#c002-guardas-de-bucles) | Guardas de Bucles | ▲ Complejidad | \* Seguro de aplicar | Media |
| [C003](#c003-extraer-funcion-auxiliar) | Extraer Función Auxiliar | ▲ Complejidad | i Informativo | Baja |
| [C004](#c004-dividir-despachador) | Dividir Despachador | ▲ Complejidad | i Informativo | Baja |
| [C005](#c005-extraer-predicado) | Extraer Predicado | ◆ Legibilidad | \* Seguro de aplicar | Baja |
| [C007](#c007-aplanar-if-anidados) | Aplanar If Anidados | ◆ Legibilidad | \* Seguro de aplicar | Máxima |
| [C011](#c011-aplanar-tryexcept) | Aplanar Try/Except | ▲ Complejidad | i Informativo | Baja |
