# Comparación con Ruff

## Resumen

Tanto [Ruff](https://docs.astral.sh/ruff/) como complexipy ayudan a mejorar la calidad del código Python, pero miden diferentes aspectos de la complejidad y sirven a propósitos complementarios. PLR0912 de Ruff mide la complejidad estructural, la densidad de pruebas y de ramas, mientras que complexipy mide qué tan difícil es el código para que los seres humanos lo entiendan y mantengan.

## PLR0912 de Ruff: Demasiadas Ramas

La regla [PLR0912 (too-many-branches)](https://docs.astral.sh/ruff/rules/too-many-branches/) de Ruff se basa en la **complejidad ciclomática** y cuenta el número de puntos de decisión en una función.

### Cómo Ruff Cuenta las Ramas

Ruff cuenta cada uno de estos como una rama:
- Sentencias `if`, `elif`, `else`
- Bucles `for` y `while`
- Operadores booleanos `and`, `or`
- Cláusulas `except`
- Casos de coincidencia de patrones

El umbral predeterminado es típicamente de **12 ramas** por función.

### Ejemplo: Análisis de Ruff

```python
def example(a, b, c, d):
    if a:           # Rama 1
        return 1
    elif b:         # Rama 2
        return 2
    elif c:         # Rama 3
        return 3
    elif d:         # Rama 4
        return 4
    else:           # Rama 5
        return 0
# Ruff: 5 ramas
```

## complexipy: Complejidad Cognitiva

complexipy implementa la **complejidad cognitiva**, que pondera las ramas según su nivel de anidamiento para reflejar la comprensión humana.

### Cómo Puntúa complexipy

complexipy usa la fórmula: `complexity = base_score + nesting_level`

```python
def example(a, b, c, d):
    if a:           # +1 (1 + 0 anidamiento)
        return 1
    elif b:         # +1 (elif cuenta como +1)
        return 2
    elif c:         # +1
        return 3
    elif d:         # +1
        return 4
    else:           # +0 (else no añade puntuación base)
        return 0
# complexipy: 4 puntos
```

## Diferencias Clave

### 1. El Anidamiento es Crítico en complexipy

**Ruff trata todas las ramas por igual:**
```python
# Ruff: 3 ramas
def flat_logic(a, b, c):
    if a:           # Rama 1
        return 1
    if b:           # Rama 2
        return 2
    if c:           # Rama 3
        return 3
```

```python
# Ruff: 3 ramas (igual que arriba)
def nested_logic(a, b, c):
    if a:           # Rama 1
        if b:       # Rama 2
            if c:   # Rama 3
                return 1
```

**complexipy tiene en cuenta el anidamiento:**
```python
# complexipy: 3 puntos
def flat_logic(a, b, c):
    if a:           # +1
        return 1
    if b:           # +1
        return 2
    if c:           # +1
        return 3
```

```python
# complexipy: 6 puntos (¡mucho peor!)
def nested_logic(a, b, c):
    if a:           # +1 (1 + 0)
        if b:       # +2 (1 + 1)
            if c:   # +3 (1 + 2)
                return 1
```

### 2. Cláusulas else

**Ruff**: Cuenta `else` como una rama (+1)

**complexipy**: `else` no añade complejidad base, solo penalización por anidamiento

```python
# Ruff: 2 ramas
# complexipy: 1 punto
def check(value):
    if value > 0:   # Ruff: +1, complexipy: +1
        return "positive"
    else:           # Ruff: +1, complexipy: +0
        return "non-positive"
```

### 3. Operadores Booleanos

Ambas cuentan los operadores booleanos, pero con filosofías diferentes:

**Ruff**: Cuenta cada `and`/`or` como una rama separada

**complexipy**: Cuenta operadores y penaliza los tipos de operador mezclados

```python
# Ruff: 3 ramas (if + and + or)
# complexipy: 3 puntos (1 por if, +2 por operadores booleanos)
def check(a, b, c):
    if a and b or c:
        return True
```

### 4. Sentencias Match (Python 3.10+)

**Ruff**: Cuenta cada `case` como una rama

**complexipy**: El match en sí no añade complejidad, solo cuenta el contenido anidado

```python
# Ruff: 3 ramas (match + 2 casos)
# complexipy: 2 puntos (solo los if anidados)
match value:        # Ruff: +1, complexipy: +0
    case 1:         # Ruff: +1, complexipy: +0
        if x:       # Ruff: +1, complexipy: +1
            pass
    case 2:         # Ya contado por Ruff
        if y:       # complexipy: +1
            pass
```

## Ejemplo de Comparación Práctica

Aquí hay un escenario del mundo real que muestra cómo difieren las dos herramientas:

```python
def process_payment(order):
    if order is None:                              # Ruff: 1, complexipy: 1
        return False

    if not order.is_valid():                       # Ruff: 2, complexipy: 2
        return False

    if order.payment_method == "credit_card":      # Ruff: 3, complexipy: 3
        if order.amount > 1000:                    # Ruff: 4, complexipy: 5 (1+1 anidamiento)
            if not verify_fraud_check(order):      # Ruff: 5, complexipy: 8 (1+2 anidamiento)
                return False
        return process_credit_card(order)
    elif order.payment_method == "paypal":         # Ruff: 6, complexipy: 4
        return process_paypal(order)
    else:                                           # Ruff: 7, complexipy: 4
        return False

# Puntuaciones Finales:
# Ruff: 7 ramas
# complexipy: 8 puntos
```

En este ejemplo:
- Ruff marca la función por tener 7 ramas (por encima del umbral típico)
- complexipy le da 8 puntos, con la mayor complejidad proveniente de la verificación de fraude anidada
- complexipy identifica mejor que la verificación de fraude profundamente anidada es la parte problemática

## ¿Cuál Deberías Usar?

### Usa Ruff (PLR0912) Cuando:
- Necesitas medir la complejidad estructural y la densidad de ramas
- Estás diseñando estrategias de cobertura de pruebas
- Quieres limitar el número absoluto de puntos de decisión
- Ya estás usando Ruff para otros tipos de linting

### Usa complexipy Cuando:
- Quieres identificar código que sea difícil de entender para los seres humanos
- Te enfocas en la legibilidad y mantenibilidad del código
- El anidamiento y la complejidad de flujo son preocupaciones en tu base de código
- Estás realizando revisiones de código enfocadas en la comprensión

### ¡Usa Ambas! (Recomendado)

Ruff y complexipy son complementarios:

```bash
# En tu pipeline de pre-commit o CI
ruff check . --select=PLR0912  # Detectar funciones con demasiadas ramas
complexipy . --max-complexity-allowed 15  # Detectar código profundamente anidado
```

**Ruff** detecta funciones anchas (muchas ramas), mientras que **complexipy** detecta funciones profundas (anidamiento intenso). Juntos, proporcionan una cobertura de complejidad integral.

## Ejemplos de Configuración

### Configuración de Ruff

```toml
# pyproject.toml
[tool.ruff]
select = ["PLR0912"]

[tool.ruff.pylint]
max-branches = 12
```

### Configuración de complexipy

```toml
# pyproject.toml
[tool.complexipy]
max-complexity-allowed = 15
```

### Usar Ambas en CI

```yaml
# .github/workflows/quality.yml
- name: Ruff Linting
  run: ruff check . --select=PLR0912

- name: Cognitive Complexity Check
  run: complexipy . --max-complexity-allowed 15
```

## Estrategia de Migración

Si ya estás usando Ruff y quieres agregar complexipy:

1. **Línea Base Primero**: Ejecuta `complexipy . --snapshot-create` para capturar el estado actual
2. **Establecer Umbrales**: Comienza con un umbral más alto (p. ej., 20) y redúcelo con el tiempo
3. **Corregir Código Nuevo**: Solo falla el CI en nuevas violaciones
4. **Mejora Gradual**: Refactoriza el código heredado de manera oportunista

## Resumen

| Característica | Ruff PLR0912 | complexipy |
|----------------|--------------|------------|
| **Basado en** | Complejidad Ciclomática | Complejidad Cognitiva |
| **Cuenta anidamiento** | ❌ No | ✅ Sí |
| **Penalización por else** | ✅ Sí | ❌ No (solo anidamiento) |
| **Operadores booleanos** | ✅ Sí | ✅ Sí |
| **Sentencias match** | ✅ Sí | Parcial (solo contenido) |
| **Ideal para** | Complejidad estructural, pruebas, densidad de ramas | Qué tan difícil es entender el código |
| **Umbral** | ~12 ramas | ~15 puntos |
| **Rendimiento** | Rápido (Rust) | Muy rápido (Rust) |

**En Conclusión**: PLR0912 de Ruff mide la complejidad estructural y la densidad de ramas (útil para pruebas y análisis), mientras que complexipy mide qué tan difícil es el código para que los seres humanos lo entiendan y mantengan, penalizando el anidamiento y las interrupciones de flujo. Ambas son valiosas, y usarlas juntas proporciona la mejor cobertura para la calidad del código.

## Lecturas Adicionales

- [Documentación de Ruff](https://docs.astral.sh/ruff/)
- [Regla PLR0912 de Ruff](https://docs.astral.sh/ruff/rules/too-many-branches/)
- [Entendiendo las Puntuaciones de complexipy](understanding-scores.md)
