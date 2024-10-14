<?php

{%- for func in ci.function_definitions() %}
{%- include "TopLevelFunctionTemplate.php" %}
{%- endfor %}

{% import "macros.php" as php %}
